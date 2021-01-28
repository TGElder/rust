use super::*;
use crate::route::{Route, RouteKey, RoutesExt};
use crate::settlement::Settlement;
use crate::traits::{SendRoutes, SendSettlements};
use commons::grid::get_corners;
use std::collections::{HashMap, HashSet};
use std::time::Duration;

pub struct GetTownTraffic<T> {
    tx: T,
}

#[async_trait]
impl<T> Processor for GetTownTraffic<T>
where
    T: SendRoutes + SendSettlements + Send + Sync,
{
    async fn process(&mut self, mut state: State, instruction: &Instruction) -> State {
        let (settlement, territory) = match instruction {
            Instruction::GetTownTraffic {
                settlement,
                territory,
            } => (settlement, territory),
            _ => return state,
        };

        let route_keys = get_route_keys(territory, &state);
        let route_to_ports = get_route_to_ports(route_keys, &state);
        let traffic_summaries = self
            .get_traffic_summaries(route_to_ports, territory.clone())
            .await;
        let aggregated_traffic_summaries = aggregate_by_nation(traffic_summaries);

        state.instructions.push(Instruction::UpdateTown {
            settlement: settlement.clone(),
            traffic: aggregated_traffic_summaries,
        });

        state
    }
}

impl<T> GetTownTraffic<T>
where
    T: SendRoutes + SendSettlements + Send,
{
    pub fn new(tx: T) -> GetTownTraffic<T> {
        GetTownTraffic { tx }
    }

    async fn get_traffic_summaries(
        &self,
        route_to_ports: HashMap<RouteKey, HashSet<V2<usize>>>,
        territory: HashSet<V2<usize>>,
    ) -> Vec<TownTrafficSummary> {
        let mut out = vec![];
        for (route_key, ports) in route_to_ports {
            if let Some(summary) = self
                .get_traffic_summary_for_route(route_key, ports, &territory)
                .await
            {
                out.push(summary)
            }
        }
        out
    }

    async fn get_traffic_summary_for_route(
        &self,
        route_key: RouteKey,
        ports: HashSet<V2<usize>>,
        territory: &HashSet<V2<usize>>,
    ) -> Option<TownTrafficSummary> {
        if territory.contains(&route_key.settlement) {
            return None;
        }
        let nation = self.get_settlement(route_key.settlement).await?.nation;
        let (ports_in_territory, ports_outside_territory): (Vec<V2<usize>>, Vec<V2<usize>>) =
            ports.into_iter().partition(|port| territory.contains(port));
        let denominator = (ports_in_territory.len() + ports_outside_territory.len() + 1) as f64;
        let is_destination = territory.contains(&route_key.destination) as usize;
        let multiplier = is_destination + ports_in_territory.len();
        if multiplier == 0 {
            return None;
        }
        let route = self.get_route(route_key).await?;
        let numerator = (route.traffic * multiplier) as f64;
        let traffic_share = numerator / denominator;

        Some(TownTrafficSummary {
            nation: nation.clone(),
            traffic_share,
            total_duration: route.duration,
        })
    }

    async fn get_settlement(&self, position: V2<usize>) -> Option<Settlement> {
        self.tx
            .send_settlements(move |settlements| {
                settlements
                    .values()
                    .find(|settlement| get_corners(&settlement.position).contains(&position))
                    .cloned()
            })
            .await
    }

    async fn get_route(&self, route_key: RouteKey) -> Option<Route> {
        self.tx
            .send_routes(move |routes| routes.get_route(&route_key).cloned())
            .await
    }
}

fn get_route_keys(territory: &HashSet<V2<usize>>, state: &State) -> HashSet<RouteKey> {
    territory
        .iter()
        .flat_map(|position| state.traffic.get(position))
        .flatten()
        .cloned()
        .collect()
}

fn get_route_to_ports(
    route_keys: HashSet<RouteKey>,
    state: &State,
) -> HashMap<RouteKey, HashSet<V2<usize>>> {
    route_keys
        .into_iter()
        .map(|route_key| {
            (
                route_key,
                state
                    .route_to_ports
                    .get(&route_key)
                    .cloned()
                    .unwrap_or_default(),
            )
        })
        .collect()
}

fn aggregate_by_nation(traffic_summaries: Vec<TownTrafficSummary>) -> Vec<TownTrafficSummary> {
    let mut nation_to_summary = hashmap! {};
    for summary in traffic_summaries {
        let aggregate = nation_to_summary
            .entry(summary.nation.clone())
            .or_insert_with(|| TownTrafficSummary {
                nation: summary.nation.clone(),
                traffic_share: 0.0,
                total_duration: Duration::from_millis(0),
            });
        aggregate.traffic_share += summary.traffic_share;
        aggregate.total_duration += summary.total_duration.mul_f64(summary.traffic_share);
    }
    nation_to_summary
        .into_iter()
        .map(|(_, traffic_summary)| traffic_summary)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::resource::Resource;
    use crate::route::{Route, Routes};
    use commons::grid::Grid;
    use commons::same_elements;
    use commons::v2;
    use futures::executor::block_on;

    use std::collections::{HashMap, HashSet};
    use std::default::Default;
    use std::sync::Mutex;
    use std::time::Duration;

    #[derive(Default)]
    struct Tx {
        routes: Mutex<Routes>,
        settlements: Mutex<HashMap<V2<usize>, Settlement>>,
    }

    #[async_trait]
    impl SendRoutes for Tx {
        async fn send_routes<F, O>(&self, function: F) -> O
        where
            O: Send + 'static,
            F: FnOnce(&mut Routes) -> O + Send + 'static,
        {
            function(&mut self.routes.lock().unwrap())
        }
    }

    #[async_trait]
    impl SendSettlements for Tx {
        async fn send_settlements<F, O>(&self, function: F) -> O
        where
            O: Send + 'static,
            F: FnOnce(&mut HashMap<V2<usize>, Settlement>) -> O + Send + 'static,
        {
            function(&mut self.settlements.lock().unwrap())
        }
    }

    fn add_route(route_key: RouteKey, route: Route, routes: &Mutex<Routes>, traffic: &mut Traffic) {
        for position in route.path.iter() {
            traffic.mut_cell_unsafe(position).insert(route_key);
        }

        let mut routes = routes.lock().unwrap();
        let route_set = routes.entry(route_key.into()).or_default();
        route_set.insert(route_key, route);
    }

    #[test]
    fn should_include_routes_ending_in_territory() {
        // Given
        let settlement = Settlement {
            position: v2(3, 1),
            ..Settlement::default()
        };
        let territory = hashset! { v2(2, 1), v2(2, 2), v2(3, 1), v2(3, 2) };

        let mut traffic = Traffic::new(5, 5, HashSet::with_capacity(0));
        let tx = Tx {
            settlements: Mutex::new(hashmap! {
                v2(0, 0) => Settlement{
                    position: v2(0, 0),
                    nation: "A".to_string(),
                    ..Settlement::default()
                },
            }),
            ..Tx::default()
        };

        let route_key = RouteKey {
            settlement: v2(0, 0),
            resource: Resource::Gems,
            destination: v2(2, 1),
        };
        let route = Route {
            path: vec![v2(0, 0), v2(1, 0), v2(2, 0), v2(2, 1)],
            traffic: 39,
            start_micros: 0,
            duration: Duration::from_millis(2),
        };
        add_route(route_key, route, &tx.routes, &mut traffic);

        let mut processor = GetTownTraffic::new(tx);

        let state = State {
            traffic,
            ..State::default()
        };
        let instruction = Instruction::GetTownTraffic {
            settlement: settlement.clone(),
            territory,
        };

        // When
        let state = block_on(processor.process(state, &instruction));

        // Then
        assert_eq!(
            state.instructions,
            vec![Instruction::UpdateTown {
                settlement,
                traffic: vec![TownTrafficSummary {
                    nation: "A".to_string(),
                    traffic_share: 39.0,
                    total_duration: Duration::from_millis(78)
                }]
            }]
        );
    }

    #[test]
    fn should_include_route_with_port_in_territory() {
        // Given
        let settlement = Settlement {
            position: v2(3, 1),
            ..Settlement::default()
        };
        let territory = hashset! { v2(2, 1), v2(2, 2), v2(3, 1), v2(3, 2) };

        let mut traffic = Traffic::new(5, 5, HashSet::with_capacity(0));
        let tx = Tx {
            settlements: Mutex::new(hashmap! {
                v2(2, 0) => Settlement{
                    position: v2(2, 0),
                    nation: "A".to_string(),
                    ..Settlement::default()
                },
            }),
            ..Tx::default()
        };

        let route_key = RouteKey {
            settlement: v2(2, 0),
            resource: Resource::Gems,
            destination: v2(2, 3),
        };
        let route = Route {
            path: vec![v2(2, 0), v2(2, 1), v2(2, 2), v2(2, 3)],
            traffic: 14,
            start_micros: 0,
            duration: Duration::from_millis(2),
        };
        add_route(route_key, route, &tx.routes, &mut traffic);

        let mut processor = GetTownTraffic::new(tx);

        let state = State {
            traffic,
            route_to_ports: hashmap! {route_key => hashset!{v2(2, 1)}},
            ..State::default()
        };
        let instruction = Instruction::GetTownTraffic {
            settlement: settlement.clone(),
            territory,
        };

        // When
        let state = block_on(processor.process(state, &instruction));

        // Then
        assert_eq!(
            state.instructions,
            vec![Instruction::UpdateTown {
                settlement,
                traffic: vec![TownTrafficSummary {
                    nation: "A".to_string(),
                    traffic_share: 7.0, // half because destination not in territory,
                    total_duration: Duration::from_millis(14)
                }]
            }]
        );
    }

    #[test]
    fn should_include_route_with_destination_and_port_in_territory() {
        // Given
        let settlement = Settlement {
            position: v2(3, 1),
            ..Settlement::default()
        };
        let territory = hashset! { v2(2, 1), v2(2, 2), v2(3, 1), v2(3, 2) };

        let mut traffic = Traffic::new(5, 5, HashSet::with_capacity(0));
        let tx = Tx {
            settlements: Mutex::new(hashmap! {
                v2(2, 0) => Settlement{
                    position: v2(2, 0),
                    nation: "A".to_string(),
                    ..Settlement::default()
                },
            }),
            ..Tx::default()
        };

        let route_key = RouteKey {
            settlement: v2(2, 0),
            resource: Resource::Gems,
            destination: v2(2, 2),
        };
        let route = Route {
            path: vec![v2(2, 0), v2(2, 1), v2(2, 2)],
            traffic: 14,
            start_micros: 0,
            duration: Duration::from_millis(2),
        };
        add_route(route_key, route, &tx.routes, &mut traffic);

        let mut processor = GetTownTraffic::new(tx);

        let state = State {
            traffic,
            route_to_ports: hashmap! {route_key => hashset!{v2(2, 1)}},
            ..State::default()
        };
        let instruction = Instruction::GetTownTraffic {
            settlement: settlement.clone(),
            territory,
        };

        // When
        let state = block_on(processor.process(state, &instruction));

        // Then
        assert_eq!(
            state.instructions,
            vec![Instruction::UpdateTown {
                settlement,
                traffic: vec![TownTrafficSummary {
                    nation: "A".to_string(),
                    traffic_share: 14.0,
                    total_duration: Duration::from_millis(28)
                }]
            }]
        );
    }

    #[test]
    fn should_include_routes_ending_in_territory_with_port_outside_territory() {
        // Given
        let settlement = Settlement {
            position: v2(3, 1),
            ..Settlement::default()
        };
        let territory = hashset! { v2(2, 1), v2(2, 2), v2(3, 1), v2(3, 2) };

        let mut traffic = Traffic::new(5, 5, HashSet::with_capacity(0));
        let tx = Tx {
            settlements: Mutex::new(hashmap! {
                v2(0, 0) => Settlement{
                    position: v2(0, 0),
                    nation: "A".to_string(),
                    ..Settlement::default()
                },
            }),
            ..Tx::default()
        };

        let route_key = RouteKey {
            settlement: v2(0, 0),
            resource: Resource::Gems,
            destination: v2(2, 1),
        };
        let route = Route {
            path: vec![v2(0, 0), v2(1, 0), v2(2, 0), v2(2, 1)],
            traffic: 14,
            start_micros: 0,
            duration: Duration::from_millis(2),
        };
        add_route(route_key, route, &tx.routes, &mut traffic);

        let mut processor = GetTownTraffic::new(tx);

        let state = State {
            traffic,
            route_to_ports: hashmap! {route_key => hashset!{v2(1, 0)}},
            ..State::default()
        };
        let instruction = Instruction::GetTownTraffic {
            settlement: settlement.clone(),
            territory,
        };

        // When
        let state = block_on(processor.process(state, &instruction));

        // Then
        assert_eq!(
            state.instructions,
            vec![Instruction::UpdateTown {
                settlement,
                traffic: vec![TownTrafficSummary {
                    nation: "A".to_string(),
                    traffic_share: 7.0, // half because port not in territory,
                    total_duration: Duration::from_millis(14)
                }]
            }]
        );
    }

    #[test]
    fn should_aggregate_routes_from_same_nation() {
        // Given
        let settlement = Settlement {
            position: v2(3, 1),
            ..Settlement::default()
        };
        let territory = hashset! { v2(2, 1), v2(2, 2), v2(3, 1), v2(3, 2) };

        let mut traffic = Traffic::new(5, 5, HashSet::with_capacity(0));
        let tx = Tx {
            settlements: Mutex::new(hashmap! {
                v2(0, 0) => Settlement{
                    position: v2(0, 0),
                    nation: "A".to_string(),
                    ..Settlement::default()
                },
                v2(3, 3) => Settlement{
                    position: v2(3, 3),
                    nation: "A".to_string(),
                    ..Settlement::default()
                },
            }),
            ..Tx::default()
        };

        let route_key_1 = RouteKey {
            settlement: v2(0, 0),
            resource: Resource::Gems,
            destination: v2(2, 1),
        };
        let route_1 = Route {
            path: vec![v2(0, 0), v2(1, 0), v2(2, 0), v2(2, 1)],
            traffic: 3,
            start_micros: 0,
            duration: Duration::from_millis(2),
        };
        add_route(route_key_1, route_1, &tx.routes, &mut traffic);

        let route_key_2 = RouteKey {
            settlement: v2(3, 3),
            resource: Resource::Gems,
            destination: v2(2, 2),
        };
        let route_2 = Route {
            path: vec![v2(3, 3), v2(3, 2), v2(2, 2)],
            traffic: 7,
            start_micros: 0,
            duration: Duration::from_millis(3),
        };
        add_route(route_key_2, route_2, &tx.routes, &mut traffic);

        let mut processor = GetTownTraffic::new(tx);

        let state = State {
            traffic,
            ..State::default()
        };
        let instruction = Instruction::GetTownTraffic {
            settlement: settlement.clone(),
            territory,
        };

        // When
        let state = block_on(processor.process(state, &instruction));

        // Then
        assert_eq!(
            state.instructions,
            vec![Instruction::UpdateTown {
                settlement,
                traffic: vec![TownTrafficSummary {
                    nation: "A".to_string(),
                    traffic_share: 10.0,
                    total_duration: Duration::from_millis(27)
                }]
            }]
        );
    }

    #[test]
    fn should_split_routes_from_different_nations() {
        // Given
        let settlement = Settlement {
            position: v2(3, 1),
            ..Settlement::default()
        };
        let territory = hashset! { v2(2, 1), v2(2, 2), v2(3, 1), v2(3, 2) };

        let mut traffic = Traffic::new(5, 5, HashSet::with_capacity(0));
        let tx = Tx {
            settlements: Mutex::new(hashmap! {
                v2(0, 0) => Settlement{
                    position: v2(0, 0),
                    nation: "A".to_string(),
                    ..Settlement::default()
                },
                v2(3, 3) => Settlement{
                    position: v2(3, 3),
                    nation: "B".to_string(),
                    ..Settlement::default()
                },
            }),
            ..Tx::default()
        };

        let route_key_1 = RouteKey {
            settlement: v2(0, 0),
            resource: Resource::Gems,
            destination: v2(2, 1),
        };
        let route_1 = Route {
            path: vec![v2(0, 0), v2(1, 0), v2(2, 0), v2(2, 1)],
            traffic: 3,
            start_micros: 0,
            duration: Duration::from_millis(2),
        };
        add_route(route_key_1, route_1, &tx.routes, &mut traffic);

        let route_key_2 = RouteKey {
            settlement: v2(3, 3),
            resource: Resource::Gems,
            destination: v2(2, 2),
        };
        let route_2 = Route {
            path: vec![v2(3, 3), v2(3, 2), v2(2, 2)],
            traffic: 7,
            start_micros: 0,
            duration: Duration::from_millis(3),
        };
        add_route(route_key_2, route_2, &tx.routes, &mut traffic);

        let mut processor = GetTownTraffic::new(tx);

        let state = State {
            traffic,
            ..State::default()
        };
        let instruction = Instruction::GetTownTraffic {
            settlement,
            territory,
        };

        // When
        let state = block_on(processor.process(state, &instruction));

        // Then
        if let Some(Instruction::UpdateTown {
            traffic: expected, ..
        }) = state.instructions.get(0)
        {
            assert!(same_elements(
                &expected,
                &[
                    TownTrafficSummary {
                        nation: "A".to_string(),
                        traffic_share: 3.0,
                        total_duration: Duration::from_millis(6),
                    },
                    TownTrafficSummary {
                        nation: "B".to_string(),
                        traffic_share: 7.0,
                        total_duration: Duration::from_millis(21),
                    }
                ]
            ));
        } else {
            panic!("No update town instruction!");
        }
    }

    #[test]
    fn should_ignore_routes_not_ending_in_territory() {
        // Given
        let settlement = Settlement {
            position: v2(3, 1),
            target_population: 10.0,
            ..Settlement::default()
        };
        let territory = hashset! { v2(2, 1), v2(2, 2), v2(3, 1), v2(3, 2) };

        let mut traffic = Traffic::new(5, 5, HashSet::with_capacity(0));
        let tx = Tx {
            settlements: Mutex::new(hashmap! {
                v2(0, 0) => Settlement{
                    position: v2(0, 0),
                    ..Settlement::default()
                },
            }),
            ..Tx::default()
        };

        let route_key = RouteKey {
            settlement: v2(0, 0),
            resource: Resource::Gems,
            destination: v2(3, 3),
        };
        let route = Route {
            path: vec![
                v2(0, 0),
                v2(1, 0),
                v2(2, 0),
                v2(3, 0),
                v2(3, 1),
                v2(3, 2),
                v2(3, 3),
            ],
            traffic: 13,
            start_micros: 0,
            duration: Duration::default(),
        };
        add_route(route_key, route, &tx.routes, &mut traffic);

        let mut processor = GetTownTraffic::new(tx);

        let state = State {
            traffic,
            ..State::default()
        };
        let instruction = Instruction::GetTownTraffic {
            settlement: settlement.clone(),
            territory,
        };

        // When
        let state = block_on(processor.process(state, &instruction));

        // Then
        assert_eq!(
            state.instructions,
            vec![Instruction::UpdateTown {
                settlement,
                traffic: vec![]
            }]
        );
    }

    #[test]
    fn should_ignore_routes_starting_in_territory() {
        // Given
        let settlement = Settlement {
            position: v2(3, 1),
            target_population: 10.0,
            ..Settlement::default()
        };
        let territory = hashset! { v2(2, 1), v2(2, 2), v2(3, 1), v2(3, 2) };

        let mut traffic = Traffic::new(5, 5, HashSet::with_capacity(0));
        let tx = Tx {
            settlements: Mutex::new(hashmap! {
                v2(3, 1) => Settlement{
                    position: v2(3, 1),
                    ..Settlement::default()
                },
            }),
            ..Tx::default()
        };
        let route_key = RouteKey {
            settlement: v2(3, 1),
            resource: Resource::Gems,
            destination: v2(3, 2),
        };
        let route = Route {
            path: vec![v2(3, 1), v2(3, 2)],
            traffic: 13,
            start_micros: 0,
            duration: Duration::default(),
        };
        add_route(route_key, route, &tx.routes, &mut traffic);

        let mut processor = GetTownTraffic::new(tx);

        let state = State {
            traffic,
            ..State::default()
        };
        let instruction = Instruction::GetTownTraffic {
            settlement: settlement.clone(),
            territory,
        };

        // When
        let state = block_on(processor.process(state, &instruction));

        // Then
        assert_eq!(
            state.instructions,
            vec![Instruction::UpdateTown {
                settlement,
                traffic: vec![]
            }]
        );
    }

    #[test]
    fn should_ignore_ports_outside_territory() {
        // Given
        let settlement = Settlement {
            position: v2(3, 1),
            target_population: 10.0,
            ..Settlement::default()
        };
        let territory = hashset! { v2(2, 0), v2(2, 1), v2(2, 2), v2(2, 3) };

        let mut traffic = Traffic::new(5, 5, HashSet::with_capacity(0));
        let tx = Tx {
            settlements: Mutex::new(hashmap! {
                v2(0, 1) => Settlement{
                    position: v2(0, 1),
                    ..Settlement::default()
                },
            }),
            ..Tx::default()
        };

        let route_key = RouteKey {
            settlement: v2(0, 1),
            resource: Resource::Gems,
            destination: v2(3, 1),
        };
        let route = Route {
            path: vec![v2(0, 1), v2(1, 1), v2(2, 1), v2(3, 1)],
            traffic: 32,
            start_micros: 0,
            duration: Duration::default(),
        };
        add_route(route_key, route, &tx.routes, &mut traffic);

        let mut processor = GetTownTraffic::new(tx);

        let state = State {
            traffic,
            route_to_ports: hashmap! {
                route_key => hashset! { v2(0, 1) }
            },
            ..State::default()
        };
        let instruction = Instruction::GetTownTraffic {
            settlement: settlement.clone(),
            territory,
        };

        // When
        let state = block_on(processor.process(state, &instruction));

        // Then
        assert_eq!(
            state.instructions,
            vec![Instruction::UpdateTown {
                settlement,
                traffic: vec![]
            }]
        );
    }

    #[test]
    fn should_ignore_invalid_route() {
        // Given
        let settlement = Settlement {
            position: v2(3, 1),
            ..Settlement::default()
        };
        let territory = hashset! { v2(2, 1), v2(2, 2), v2(3, 1), v2(3, 2) };

        let mut traffic = Traffic::new(5, 5, HashSet::with_capacity(0));
        let mut tx = Tx {
            settlements: Mutex::new(hashmap! {
                v2(0, 0) => Settlement{
                    position: v2(0, 0),
                    nation: "A".to_string(),
                    ..Settlement::default()
                },
            }),
            ..Tx::default()
        };

        let route_key = RouteKey {
            settlement: v2(0, 0),
            resource: Resource::Gems,
            destination: v2(2, 1),
        };
        let route = Route {
            path: vec![v2(0, 0), v2(1, 0), v2(2, 0), v2(2, 1)],
            traffic: 39,
            start_micros: 0,
            duration: Duration::default(),
        };
        add_route(route_key, route, &tx.routes, &mut traffic);
        tx.routes = Mutex::default(); // Removing route to create invalid state

        let mut processor = GetTownTraffic::new(tx);

        let state = State {
            traffic,
            ..State::default()
        };
        let instruction = Instruction::GetTownTraffic {
            settlement: settlement.clone(),
            territory,
        };

        // When
        let state = block_on(processor.process(state, &instruction));

        // Then
        assert_eq!(
            state.instructions,
            vec![Instruction::UpdateTown {
                settlement,
                traffic: vec![]
            }]
        );
    }

    #[test]
    fn should_link_route_from_corner_of_settlement_to_correct_settlement() {
        // Given
        let settlement = Settlement {
            position: v2(3, 1),
            ..Settlement::default()
        };
        let territory = hashset! { v2(2, 1), v2(2, 2), v2(3, 1), v2(3, 2) };

        let mut traffic = Traffic::new(5, 5, HashSet::with_capacity(0));
        let tx = Tx {
            settlements: Mutex::new(hashmap! {
                v2(0, 0) => Settlement{
                    position: v2(0, 0),
                    nation: "A".to_string(),
                    ..Settlement::default()
                },
            }),
            ..Tx::default()
        };

        let route_key = RouteKey {
            settlement: v2(1, 1),
            resource: Resource::Gems,
            destination: v2(2, 2),
        };
        let route = Route {
            path: vec![v2(1, 1), v2(2, 1), v2(2, 2)],
            traffic: 10,
            start_micros: 0,
            duration: Duration::from_millis(2),
        };
        add_route(route_key, route, &tx.routes, &mut traffic);

        let mut processor = GetTownTraffic::new(tx);

        let state = State {
            traffic,
            ..State::default()
        };
        let instruction = Instruction::GetTownTraffic {
            settlement: settlement.clone(),
            territory,
        };

        // When
        let state = block_on(processor.process(state, &instruction));

        // Then
        assert_eq!(
            state.instructions,
            vec![Instruction::UpdateTown {
                settlement,
                traffic: vec![TownTrafficSummary {
                    nation: "A".to_string(),
                    traffic_share: 10.0,
                    total_duration: Duration::from_millis(20),
                }]
            }]
        );
    }

    #[test]
    fn should_ignore_route_from_invalid_settlement() {
        // Given
        let settlement = Settlement {
            position: v2(3, 1),
            ..Settlement::default()
        };
        let territory = hashset! { v2(2, 1), v2(2, 2), v2(3, 1), v2(3, 2) };

        let mut traffic = Traffic::new(5, 5, HashSet::with_capacity(0));
        let tx = Tx::default();

        let route_key = RouteKey {
            settlement: v2(0, 0),
            resource: Resource::Gems,
            destination: v2(2, 1),
        };
        let route = Route {
            path: vec![v2(0, 0), v2(1, 0), v2(2, 0), v2(2, 1)],
            traffic: 39,
            start_micros: 0,
            duration: Duration::default(),
        };
        add_route(route_key, route, &tx.routes, &mut traffic);

        let mut processor = GetTownTraffic::new(tx);

        let state = State {
            traffic,
            ..State::default()
        };
        let instruction = Instruction::GetTownTraffic {
            settlement: settlement.clone(),
            territory,
        };

        // When
        let state = block_on(processor.process(state, &instruction));

        // Then
        assert_eq!(
            state.instructions,
            vec![Instruction::UpdateTown {
                settlement,
                traffic: vec![]
            }]
        );
    }
}
