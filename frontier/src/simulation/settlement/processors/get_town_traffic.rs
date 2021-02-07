use super::*;
use crate::route::{Route, RouteKey, RoutesExt};
use crate::settlement::Settlement;
use crate::traits::{SendRoutes, SendSettlements, WithRouteToPorts, WithTraffic};
use commons::grid::get_corners;
use std::collections::{HashMap, HashSet};
use std::time::Duration;

pub struct GetTownTraffic<T> {
    tx: T,
}

impl<T> GetTownTraffic<T>
where
    T: SendRoutes + SendSettlements + WithRouteToPorts + WithTraffic,
{
    pub fn new(tx: T) -> GetTownTraffic<T> {
        GetTownTraffic { tx }
    }

    pub async fn get_town_traffic(
        &self,
        territory: &HashSet<V2<usize>>,
    ) -> Vec<TownTrafficSummary> {
        let route_keys = self.get_route_keys(territory).await;
        let route_to_ports = self.get_route_to_ports(route_keys).await;
        let traffic_summaries = self
            .get_traffic_summaries(route_to_ports, territory.clone())
            .await;
        aggregate_by_nation(traffic_summaries)
    }

    async fn get_route_keys(&self, territory: &HashSet<V2<usize>>) -> HashSet<RouteKey> {
        self.tx
            .with_traffic(|traffic| get_route_keys(traffic, territory))
            .await
    }

    async fn get_route_to_ports(
        &self,
        keys: HashSet<RouteKey>,
    ) -> HashMap<RouteKey, HashSet<V2<usize>>> {
        self.tx
            .with_route_to_ports(|route_to_ports| get_route_to_ports(route_to_ports, keys))
            .await
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

fn get_route_keys(traffic: &Traffic, territory: &HashSet<V2<usize>>) -> HashSet<RouteKey> {
    territory
        .iter()
        .flat_map(|position| traffic.get(position))
        .flatten()
        .cloned()
        .collect()
}

fn get_route_to_ports(
    route_to_ports: &HashMap<RouteKey, HashSet<V2<usize>>>,
    route_keys: HashSet<RouteKey>,
) -> HashMap<RouteKey, HashSet<V2<usize>>> {
    route_keys
        .into_iter()
        .map(|route_key| {
            (
                route_key,
                route_to_ports.get(&route_key).cloned().unwrap_or_default(),
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

    use std::collections::HashMap;
    use std::default::Default;
    use std::sync::Mutex;
    use std::time::Duration;

    struct Tx {
        route_to_ports: Mutex<HashMap<RouteKey, HashSet<V2<usize>>>>,
        routes: Mutex<Routes>,
        settlements: Mutex<HashMap<V2<usize>, Settlement>>,
        traffic: Mutex<Traffic>,
    }

    impl Default for Tx {
        fn default() -> Self {
            Tx {
                route_to_ports: Mutex::default(),
                routes: Mutex::default(),
                settlements: Mutex::default(),
                traffic: Mutex::new(Traffic::new(5, 5, hashset! {})),
            }
        }
    }

    impl Tx {
        fn add_route(&self, route_key: RouteKey, route: Route) {
            for position in route.path.iter() {
                self.traffic
                    .lock()
                    .unwrap()
                    .mut_cell_unsafe(position)
                    .insert(route_key);
            }

            let mut routes = self.routes.lock().unwrap();
            let route_set = routes.entry(route_key.into()).or_default();
            route_set.insert(route_key, route);
        }
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

    #[async_trait]
    impl WithRouteToPorts for Tx {
        async fn with_route_to_ports<F, O>(&self, function: F) -> O
        where
            F: FnOnce(&HashMap<RouteKey, HashSet<V2<usize>>>) -> O + Send,
        {
            function(&self.route_to_ports.lock().unwrap())
        }

        async fn mut_route_to_ports<F, O>(&self, function: F) -> O
        where
            F: FnOnce(&mut HashMap<RouteKey, HashSet<V2<usize>>>) -> O + Send,
        {
            function(&mut self.route_to_ports.lock().unwrap())
        }
    }

    #[async_trait]
    impl WithTraffic for Tx {
        async fn with_traffic<F, O>(&self, function: F) -> O
        where
            F: FnOnce(&Traffic) -> O + Send,
        {
            function(&self.traffic.lock().unwrap())
        }

        async fn mut_traffic<F, O>(&self, function: F) -> O
        where
            F: FnOnce(&mut Traffic) -> O + Send,
        {
            function(&mut self.traffic.lock().unwrap())
        }
    }

    #[test]
    fn should_include_routes_ending_in_territory() {
        // Given
        let territory = hashset! { v2(2, 1), v2(2, 2), v2(3, 1), v2(3, 2) };

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
        tx.add_route(route_key, route);

        let processor = GetTownTraffic::new(tx);

        // When
        let traffic_summaries = block_on(processor.get_town_traffic(&territory));

        // Then
        assert_eq!(
            traffic_summaries,
            vec![TownTrafficSummary {
                nation: "A".to_string(),
                traffic_share: 39.0,
                total_duration: Duration::from_millis(78)
            }]
        );
    }

    #[test]
    fn should_include_route_with_port_in_territory() {
        // Given
        let territory = hashset! { v2(2, 1), v2(2, 2), v2(3, 1), v2(3, 2) };

        let route_key = RouteKey {
            settlement: v2(2, 0),
            resource: Resource::Gems,
            destination: v2(2, 3),
        };

        let tx = Tx {
            route_to_ports: Mutex::new(hashmap! {route_key => hashset!{v2(2, 1)}}),
            settlements: Mutex::new(hashmap! {
                v2(2, 0) => Settlement{
                    position: v2(2, 0),
                    nation: "A".to_string(),
                    ..Settlement::default()
                },
            }),
            ..Tx::default()
        };

        let route = Route {
            path: vec![v2(2, 0), v2(2, 1), v2(2, 2), v2(2, 3)],
            traffic: 14,
            start_micros: 0,
            duration: Duration::from_millis(2),
        };
        tx.add_route(route_key, route);

        let processor = GetTownTraffic::new(tx);

        // When
        let traffic_summaries = block_on(processor.get_town_traffic(&territory));

        // Then
        assert_eq!(
            traffic_summaries,
            vec![TownTrafficSummary {
                nation: "A".to_string(),
                traffic_share: 7.0, // half because destination not in territory,
                total_duration: Duration::from_millis(14)
            }]
        );
    }

    #[test]
    fn should_include_route_with_destination_and_port_in_territory() {
        // Given
        let territory = hashset! { v2(2, 1), v2(2, 2), v2(3, 1), v2(3, 2) };

        let route_key = RouteKey {
            settlement: v2(2, 0),
            resource: Resource::Gems,
            destination: v2(2, 2),
        };

        let tx = Tx {
            route_to_ports: Mutex::new(hashmap! {route_key => hashset!{v2(2, 1)}}),
            settlements: Mutex::new(hashmap! {
                v2(2, 0) => Settlement{
                    position: v2(2, 0),
                    nation: "A".to_string(),
                    ..Settlement::default()
                },
            }),
            ..Tx::default()
        };

        let route = Route {
            path: vec![v2(2, 0), v2(2, 1), v2(2, 2)],
            traffic: 14,
            start_micros: 0,
            duration: Duration::from_millis(2),
        };
        tx.add_route(route_key, route);

        let processor = GetTownTraffic::new(tx);

        // When
        let traffic_summaries = block_on(processor.get_town_traffic(&territory));

        // Then
        assert_eq!(
            traffic_summaries,
            vec![TownTrafficSummary {
                nation: "A".to_string(),
                traffic_share: 14.0,
                total_duration: Duration::from_millis(28)
            }]
        );
    }

    #[test]
    fn should_include_routes_ending_in_territory_with_port_outside_territory() {
        // Given
        let territory = hashset! { v2(2, 1), v2(2, 2), v2(3, 1), v2(3, 2) };

        let route_key = RouteKey {
            settlement: v2(0, 0),
            resource: Resource::Gems,
            destination: v2(2, 1),
        };

        let tx = Tx {
            route_to_ports: Mutex::new(hashmap! {route_key => hashset!{v2(1, 0)}}),
            settlements: Mutex::new(hashmap! {
                v2(0, 0) => Settlement{
                    position: v2(0, 0),
                    nation: "A".to_string(),
                    ..Settlement::default()
                },
            }),
            ..Tx::default()
        };

        let route = Route {
            path: vec![v2(0, 0), v2(1, 0), v2(2, 0), v2(2, 1)],
            traffic: 14,
            start_micros: 0,
            duration: Duration::from_millis(2),
        };
        tx.add_route(route_key, route);

        let processor = GetTownTraffic::new(tx);

        // When
        let traffic_summaries = block_on(processor.get_town_traffic(&territory));

        // Then
        assert_eq!(
            traffic_summaries,
            vec![TownTrafficSummary {
                nation: "A".to_string(),
                traffic_share: 7.0, // half because port not in territory,
                total_duration: Duration::from_millis(14)
            }]
        );
    }

    #[test]
    fn should_aggregate_routes_from_same_nation() {
        // Given
        let territory = hashset! { v2(2, 1), v2(2, 2), v2(3, 1), v2(3, 2) };

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
        tx.add_route(route_key_1, route_1);

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
        tx.add_route(route_key_2, route_2);

        let processor = GetTownTraffic::new(tx);

        // When
        let traffic_summaries = block_on(processor.get_town_traffic(&territory));

        // Then
        assert_eq!(
            traffic_summaries,
            vec![TownTrafficSummary {
                nation: "A".to_string(),
                traffic_share: 10.0,
                total_duration: Duration::from_millis(27)
            }]
        );
    }

    #[test]
    fn should_split_routes_from_different_nations() {
        // Given
        let territory = hashset! { v2(2, 1), v2(2, 2), v2(3, 1), v2(3, 2) };

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
        tx.add_route(route_key_1, route_1);

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
        tx.add_route(route_key_2, route_2);

        let processor = GetTownTraffic::new(tx);

        // When
        let traffic_summaries = block_on(processor.get_town_traffic(&territory));

        // Then

        assert!(same_elements(
            &traffic_summaries,
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
    }

    #[test]
    fn should_ignore_routes_not_ending_in_territory() {
        // Given
        let territory = hashset! { v2(2, 1), v2(2, 2), v2(3, 1), v2(3, 2) };

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
        tx.add_route(route_key, route);

        let processor = GetTownTraffic::new(tx);

        // When
        let traffic_summaries = block_on(processor.get_town_traffic(&territory));

        // Then
        assert_eq!(traffic_summaries, vec![]);
    }

    #[test]
    fn should_ignore_routes_starting_in_territory() {
        // Given
        let territory = hashset! { v2(2, 1), v2(2, 2), v2(3, 1), v2(3, 2) };

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
        tx.add_route(route_key, route);

        let processor = GetTownTraffic::new(tx);

        // When
        let traffic_summaries = block_on(processor.get_town_traffic(&territory));

        // Then
        assert_eq!(traffic_summaries, vec![]);
    }

    #[test]
    fn should_ignore_ports_outside_territory() {
        // Given
        let territory = hashset! { v2(2, 0), v2(2, 1), v2(2, 2), v2(2, 3) };

        let route_key = RouteKey {
            settlement: v2(0, 1),
            resource: Resource::Gems,
            destination: v2(3, 1),
        };

        let tx = Tx {
            route_to_ports: Mutex::new(hashmap! {
                route_key => hashset! { v2(0, 1) }
            }),
            settlements: Mutex::new(hashmap! {
                v2(0, 1) => Settlement{
                    position: v2(0, 1),
                    ..Settlement::default()
                },
            }),
            ..Tx::default()
        };
        let route = Route {
            path: vec![v2(0, 1), v2(1, 1), v2(2, 1), v2(3, 1)],
            traffic: 32,
            start_micros: 0,
            duration: Duration::default(),
        };
        tx.add_route(route_key, route);

        let processor = GetTownTraffic::new(tx);

        // When
        let traffic_summaries = block_on(processor.get_town_traffic(&territory));

        // Then
        assert_eq!(traffic_summaries, vec![]);
    }

    #[test]
    fn should_ignore_invalid_route() {
        // Given
        let territory = hashset! { v2(2, 1), v2(2, 2), v2(3, 1), v2(3, 2) };

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
        tx.add_route(route_key, route);
        tx.routes = Mutex::default(); // Removing route to create invalid state

        let processor = GetTownTraffic::new(tx);

        // When
        let traffic_summaries = block_on(processor.get_town_traffic(&territory));

        // Then
        assert_eq!(traffic_summaries, vec![]);
    }

    #[test]
    fn should_link_route_from_corner_of_settlement_to_correct_settlement() {
        // Given
        let territory = hashset! { v2(2, 1), v2(2, 2), v2(3, 1), v2(3, 2) };

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
        tx.add_route(route_key, route);

        let processor = GetTownTraffic::new(tx);

        // When
        let traffic_summaries = block_on(processor.get_town_traffic(&territory));

        // Then
        assert_eq!(
            traffic_summaries,
            vec![TownTrafficSummary {
                nation: "A".to_string(),
                traffic_share: 10.0,
                total_duration: Duration::from_millis(20),
            }]
        );
    }

    #[test]
    fn should_ignore_route_from_invalid_settlement() {
        // Given
        let territory = hashset! { v2(2, 1), v2(2, 2), v2(3, 1), v2(3, 2) };

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
        tx.add_route(route_key, route);

        let processor = GetTownTraffic::new(tx);

        // When
        let traffic_summaries = block_on(processor.get_town_traffic(&territory));

        // Then
        assert_eq!(traffic_summaries, vec![]);
    }
}
