use super::*;
use crate::game::traits::{GetRoute, HasWorld, Settlements};
use crate::route::RouteKey;
use crate::settlement::Settlement;
use commons::get_corners;
use std::collections::{HashMap, HashSet};

const HANDLE: &str = "get_town_traffic";

pub struct GetTownTraffic<G>
where
    G: HasWorld + GetRoute + Settlements,
{
    game: UpdateSender<G>,
}

#[async_trait]
impl<G> Processor for GetTownTraffic<G>
where
    G: HasWorld + GetRoute + Settlements,
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

impl<G> GetTownTraffic<G>
where
    G: HasWorld + GetRoute + Settlements,
{
    pub fn new(game: &UpdateSender<G>) -> GetTownTraffic<G> {
        GetTownTraffic {
            game: game.clone_with_handle(HANDLE),
        }
    }

    async fn get_traffic_summaries(
        &mut self,
        route_to_ports: HashMap<RouteKey, HashSet<V2<usize>>>,
        territory: HashSet<V2<usize>>,
    ) -> Vec<TownTrafficSummary> {
        self.game
            .update(move |game| get_traffic_summaries(game, route_to_ports, territory))
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

fn get_traffic_summaries<G>(
    game: &mut G,
    route_to_ports: HashMap<RouteKey, HashSet<V2<usize>>>,
    territory: HashSet<V2<usize>>,
) -> Vec<TownTrafficSummary>
where
    G: HasWorld + GetRoute + Settlements,
{
    route_to_ports
        .into_iter()
        .flat_map(|(route_key, ports)| {
            get_traffic_summary_for_route(game, route_key, ports, &territory)
        })
        .collect()
}

fn get_traffic_summary_for_route<G>(
    game: &G,
    route_key: RouteKey,
    ports: HashSet<V2<usize>>,
    territory: &HashSet<V2<usize>>,
) -> Option<TownTrafficSummary>
where
    G: HasWorld + GetRoute + Settlements,
{
    if territory.contains(&route_key.settlement) {
        return None;
    }
    let nation = &get_settlement(game, &route_key.settlement)?.nation;
    let (ports_in_territory, ports_outside_territory): (Vec<V2<usize>>, Vec<V2<usize>>) =
        ports.into_iter().partition(|port| territory.contains(port));
    let denominator = (ports_in_territory.len() + ports_outside_territory.len() + 1) as f64;
    let is_destination = territory.contains(&route_key.destination) as usize;
    let multiplier = is_destination + ports_in_territory.len();
    if multiplier == 0 {
        return None;
    }
    let route = game.get_route(&route_key)?;
    let numerator = (route.traffic * multiplier) as f64;
    let traffic_share = numerator / denominator;

    Some(TownTrafficSummary {
        nation: nation.clone(),
        traffic_share,
    })
}

fn get_settlement<'a, G>(game: &'a G, position: &V2<usize>) -> Option<&'a Settlement>
where
    G: Settlements,
{
    game.settlements()
        .values()
        .find(|settlement| get_corners(&settlement.position).contains(position))
}

fn aggregate_by_nation(traffic_summaries: Vec<TownTrafficSummary>) -> Vec<TownTrafficSummary> {
    let mut nation_to_summary = hashmap! {};
    for summary in traffic_summaries {
        nation_to_summary
            .entry(summary.nation.clone())
            .or_insert_with(|| TownTrafficSummary {
                nation: summary.nation,
                traffic_share: 0.0,
            })
            .traffic_share += summary.traffic_share;
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
    use crate::route::Route;
    use crate::world::World;
    use commons::grid::Grid;
    use commons::same_elements;
    use commons::update::UpdateProcess;
    use commons::{v2, M};

    use std::collections::{HashMap, HashSet};
    use std::default::Default;
    use std::time::Duration;

    struct MockGame {
        routes: HashMap<RouteKey, Route>,
        settlements: HashMap<V2<usize>, Settlement>,
        world: World,
    }

    impl Default for MockGame {
        fn default() -> MockGame {
            MockGame {
                routes: hashmap! {},
                settlements: hashmap! {},
                world: World::new(M::zeros(4, 4), -0.5),
            }
        }
    }

    impl HasWorld for MockGame {
        fn world(&self) -> &World {
            &self.world
        }

        fn world_mut(&mut self) -> &mut World {
            &mut self.world
        }
    }

    impl GetRoute for MockGame {
        fn get_route(&self, route_key: &RouteKey) -> Option<&Route> {
            self.routes.get(route_key)
        }
    }

    impl Settlements for MockGame {
        fn settlements(&self) -> &HashMap<V2<usize>, Settlement> {
            &self.settlements
        }
    }

    fn add_route(
        route_key: RouteKey,
        route: Route,
        routes: &mut HashMap<RouteKey, Route>,
        traffic: &mut Traffic,
    ) {
        for position in route.path.iter() {
            traffic.mut_cell_unsafe(position).insert(route_key);
        }

        routes.insert(route_key, route);
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
        let mut game = MockGame {
            settlements: hashmap! {
                v2(0, 0) => Settlement{
                    position: v2(0, 0),
                    nation: "A".to_string(),
                    ..Settlement::default()
                },
            },
            ..MockGame::default()
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
        add_route(route_key, route, &mut game.routes, &mut traffic);

        let game = UpdateProcess::new(game);
        let mut processor = GetTownTraffic::new(&game.tx());

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
                    traffic_share: 39.0
                }]
            }]
        );

        // Finally
        game.shutdown();
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
        let mut game = MockGame {
            settlements: hashmap! {
                v2(2, 0) => Settlement{
                    position: v2(2, 0),
                    nation: "A".to_string(),
                    ..Settlement::default()
                },
            },
            ..MockGame::default()
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
            duration: Duration::default(),
        };
        add_route(route_key, route, &mut game.routes, &mut traffic);

        let game = UpdateProcess::new(game);
        let mut processor = GetTownTraffic::new(&game.tx());

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
                    traffic_share: 7.0 // half because destination not in territory
                }]
            }]
        );

        // Finally
        game.shutdown();
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
        let mut game = MockGame {
            settlements: hashmap! {
                v2(2, 0) => Settlement{
                    position: v2(2, 0),
                    nation: "A".to_string(),
                    ..Settlement::default()
                },
            },
            ..MockGame::default()
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
            duration: Duration::default(),
        };
        add_route(route_key, route, &mut game.routes, &mut traffic);

        let game = UpdateProcess::new(game);
        let mut processor = GetTownTraffic::new(&game.tx());

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
                    traffic_share: 14.0
                }]
            }]
        );

        // Finally
        game.shutdown();
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
        let mut game = MockGame {
            settlements: hashmap! {
                v2(0, 0) => Settlement{
                    position: v2(0, 0),
                    nation: "A".to_string(),
                    ..Settlement::default()
                },
            },
            ..MockGame::default()
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
            duration: Duration::default(),
        };
        add_route(route_key, route, &mut game.routes, &mut traffic);

        let game = UpdateProcess::new(game);
        let mut processor = GetTownTraffic::new(&game.tx());

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
                    traffic_share: 7.0 // half because port not in territory
                }]
            }]
        );

        // Finally
        game.shutdown();
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
        let mut game = MockGame {
            settlements: hashmap! {
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
            },
            ..MockGame::default()
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
            duration: Duration::default(),
        };
        add_route(route_key_1, route_1, &mut game.routes, &mut traffic);

        let route_key_2 = RouteKey {
            settlement: v2(3, 3),
            resource: Resource::Gems,
            destination: v2(2, 2),
        };
        let route_2 = Route {
            path: vec![v2(3, 3), v2(3, 2), v2(2, 2)],
            traffic: 7,
            start_micros: 0,
            duration: Duration::default(),
        };
        add_route(route_key_2, route_2, &mut game.routes, &mut traffic);

        let game = UpdateProcess::new(game);
        let mut processor = GetTownTraffic::new(&game.tx());

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
                    traffic_share: 10.0
                }]
            }]
        );

        // Finally
        game.shutdown();
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
        let mut game = MockGame {
            settlements: hashmap! {
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
            },
            ..MockGame::default()
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
            duration: Duration::default(),
        };
        add_route(route_key_1, route_1, &mut game.routes, &mut traffic);

        let route_key_2 = RouteKey {
            settlement: v2(3, 3),
            resource: Resource::Gems,
            destination: v2(2, 2),
        };
        let route_2 = Route {
            path: vec![v2(3, 3), v2(3, 2), v2(2, 2)],
            traffic: 7,
            start_micros: 0,
            duration: Duration::default(),
        };
        add_route(route_key_2, route_2, &mut game.routes, &mut traffic);

        let game = UpdateProcess::new(game);
        let mut processor = GetTownTraffic::new(&game.tx());

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
                        traffic_share: 3.0
                    },
                    TownTrafficSummary {
                        nation: "B".to_string(),
                        traffic_share: 7.0
                    }
                ]
            ));
        } else {
            panic!("No update town instruction!");
        }

        // Finally
        game.shutdown();
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
        let mut game = MockGame {
            settlements: hashmap! {
                v2(0, 0) => Settlement{
                    position: v2(0, 0),
                    ..Settlement::default()
                },
            },
            ..MockGame::default()
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
        add_route(route_key, route, &mut game.routes, &mut traffic);

        let game = UpdateProcess::new(game);
        let mut processor = GetTownTraffic::new(&game.tx());

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
        let mut game = MockGame {
            settlements: hashmap! {
                v2(3, 1) => Settlement{
                    position: v2(3, 1),
                    ..Settlement::default()
                },
            },
            ..MockGame::default()
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
        add_route(route_key, route, &mut game.routes, &mut traffic);

        let game = UpdateProcess::new(game);
        let mut processor = GetTownTraffic::new(&game.tx());

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

        // Finally
        game.shutdown();
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
        let mut game = MockGame {
            settlements: hashmap! {
                v2(0, 1) => Settlement{
                    position: v2(0, 1),
                    ..Settlement::default()
                },
            },
            ..MockGame::default()
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
        add_route(route_key, route, &mut game.routes, &mut traffic);

        let game = UpdateProcess::new(game);
        let mut processor = GetTownTraffic::new(&game.tx());

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

        // Finally
        game.shutdown();
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
        let mut game = MockGame {
            settlements: hashmap! {
                v2(0, 0) => Settlement{
                    position: v2(0, 0),
                    nation: "A".to_string(),
                    ..Settlement::default()
                },
            },
            ..MockGame::default()
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
        add_route(route_key, route, &mut game.routes, &mut traffic);
        game.routes = hashmap! {}; // Removing route to create invalid state

        let game = UpdateProcess::new(game);
        let mut processor = GetTownTraffic::new(&game.tx());

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

        // Finally
        game.shutdown();
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
        let mut game = MockGame {
            settlements: hashmap! {
                v2(0, 0) => Settlement{
                    position: v2(0, 0),
                    nation: "A".to_string(),
                    ..Settlement::default()
                },
            },
            ..MockGame::default()
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
            duration: Duration::default(),
        };
        add_route(route_key, route, &mut game.routes, &mut traffic);

        let game = UpdateProcess::new(game);
        let mut processor = GetTownTraffic::new(&game.tx());

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
                    traffic_share: 10.0
                }]
            }]
        );

        // Finally
        game.shutdown();
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
        let mut game = MockGame::default();

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
        add_route(route_key, route, &mut game.routes, &mut traffic);

        let game = UpdateProcess::new(game);
        let mut processor = GetTownTraffic::new(&game.tx());

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

        // Finally
        game.shutdown();
    }
}
