use crate::bridges::{BridgeDurationFn, Bridges};
use crate::route::RouteKey;
use crate::simulation::settlement::model::RouteChange;
use crate::simulation::settlement::SettlementSimulation;
use crate::traits::has::HasParameters;
use crate::traits::{WithBridges, WithRouteToGates};
use commons::edge::{Edge, Edges};
use commons::V2;
use std::collections::{HashMap, HashSet};

impl<T, D> SettlementSimulation<T, D>
where
    T: HasParameters + WithBridges + WithRouteToGates,
{
    pub async fn update_route_to_gates(&self, route_changes: &[RouteChange]) {
        let updated = get_all_updated(route_changes);
        self.remove_removed(route_changes).await;
        let gates = self.get_all_gates(&updated).await;
        self.update_gates(gates).await;
    }

    async fn remove_removed(&self, route_changes: &[RouteChange]) {
        let removed = get_all_removed(route_changes);
        self.cx
            .mut_route_to_gates(|route_to_gates| {
                for key in removed {
                    route_to_gates.remove(key);
                }
            })
            .await;
    }

    async fn get_all_gates(
        &self,
        routes: &HashMap<RouteKey, Vec<V2<usize>>>,
    ) -> HashMap<RouteKey, HashSet<V2<usize>>> {
        let bridge_duration_fn = &self.cx.parameters().npc_bridge_duration_fn;
        self.cx
            .with_bridges(|bridges| get_all_gates(bridges, bridge_duration_fn, routes))
            .await
    }

    async fn update_gates(&self, gates: HashMap<RouteKey, HashSet<V2<usize>>>) {
        self.cx
            .mut_route_to_gates(|route_to_gates| {
                for (key, gates) in gates {
                    if gates.is_empty() {
                        route_to_gates.remove(&key);
                    } else {
                        route_to_gates.insert(key, gates);
                    }
                }
            })
            .await;
    }
}

fn get_all_removed(route_changes: &[RouteChange]) -> Vec<&RouteKey> {
    route_changes.iter().flat_map(get_removed).collect()
}

fn get_removed(route_change: &RouteChange) -> Option<&RouteKey> {
    if let RouteChange::Removed { key, .. } = route_change {
        Some(key)
    } else {
        None
    }
}

fn get_all_updated(route_changes: &[RouteChange]) -> HashMap<RouteKey, Vec<V2<usize>>> {
    route_changes.iter().flat_map(get_updated).collect()
}

fn get_updated(route_change: &RouteChange) -> Option<(RouteKey, Vec<V2<usize>>)> {
    match route_change {
        RouteChange::New { key, route } => Some((*key, route.path.clone())),
        RouteChange::Updated { key, new, old } if new.path != old.path => {
            Some((*key, new.path.clone()))
        }
        RouteChange::Removed { .. } => None,
        _ => None,
    }
}

fn get_all_gates(
    bridges: &Bridges,
    bridge_duration_fn: &BridgeDurationFn,
    routes: &HashMap<RouteKey, Vec<V2<usize>>>,
) -> HashMap<RouteKey, HashSet<V2<usize>>> {
    routes
        .iter()
        .map(|(key, path)| (*key, get_gates(bridges, bridge_duration_fn, path)))
        .collect()
}

fn get_gates(
    bridges: &Bridges,
    bridge_duration_fn: &BridgeDurationFn,
    path: &[V2<usize>],
) -> HashSet<V2<usize>> {
    path.edges()
        .flat_map(|edge| check_for_gate(bridges, bridge_duration_fn, &edge))
        .collect()
}

fn check_for_gate(
    bridges: &Bridges,
    bridge_duration_fn: &BridgeDurationFn,
    edge: &Edge,
) -> Vec<V2<usize>> {
    bridges
        .get(edge)
        .and_then(|bridges| bridge_duration_fn.lowest_duration_bridge(bridges))
        .map(|bridge| vec![bridge.start().position, bridge.end().position])
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::avatar::{Rotation, Vehicle};
    use crate::bridges::{Bridge, BridgeType, Pier};
    use crate::parameters::Parameters;
    use crate::resource::Resource;
    use crate::route::Route;
    use commons::async_trait::async_trait;
    use commons::v2;
    use futures::executor::block_on;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    struct Cx {
        bridges: Mutex<Bridges>,
        parameters: Parameters,
        route_to_gates: Mutex<HashMap<RouteKey, HashSet<V2<usize>>>>,
    }

    impl HasParameters for Cx {
        fn parameters(&self) -> &Parameters {
            &self.parameters
        }
    }

    #[async_trait]
    impl WithBridges for Cx {
        async fn with_bridges<F, O>(&self, function: F) -> O
        where
            F: FnOnce(&Bridges) -> O + Send,
        {
            function(&self.bridges.lock().unwrap())
        }

        async fn mut_bridges<F, O>(&self, function: F) -> O
        where
            F: FnOnce(&mut Bridges) -> O + Send,
        {
            function(&mut self.bridges.lock().unwrap())
        }
    }

    #[async_trait]
    impl WithRouteToGates for Cx {
        async fn with_route_to_gates<F, O>(&self, function: F) -> O
        where
            F: FnOnce(&HashMap<RouteKey, HashSet<V2<usize>>>) -> O + Send,
        {
            function(&self.route_to_gates.lock().unwrap())
        }

        async fn mut_route_to_gates<F, O>(&self, function: F) -> O
        where
            F: FnOnce(&mut HashMap<RouteKey, HashSet<V2<usize>>>) -> O + Send,
        {
            function(&mut self.route_to_gates.lock().unwrap())
        }
    }

    fn cx() -> Cx {
        Cx {
            bridges: Mutex::new(Bridges::default()),
            parameters: Parameters::default(),
            route_to_gates: Mutex::default(),
        }
    }

    fn cx_with_bridges(edges: &[Edge]) -> Cx {
        let bridges = edges
            .iter()
            .map(|edge| {
                (
                    *edge,
                    hashset! {Bridge{ piers: vec![
                        Pier{
                            position: *edge.from(),
                            elevation: 0.0,
                            platform: false,
                            rotation: Rotation::Up,
                            vehicle: Vehicle::None,
                        },
                        Pier{
                            position: *edge.to(),
                            elevation: 0.0,
                            platform: false,
                            rotation: Rotation::Up,
                            vehicle: Vehicle::None,
                        }
                    ], bridge_type: BridgeType::Built }},
                )
            })
            .collect();
        Cx {
            bridges: Mutex::new(bridges),
            ..cx()
        }
    }

    #[test]
    fn should_insert_entry_for_new_route_with_gates() {
        // Given
        let key = RouteKey {
            settlement: v2(0, 0),
            resource: Resource::Truffles,
            destination: v2(2, 2),
        };
        let route = Route {
            path: vec![v2(0, 0), v2(0, 1), v2(0, 2), v2(1, 2), v2(2, 2)],
            start_micros: 0,
            duration: Duration::from_secs(0),
            traffic: 0,
        };

        let route_change = RouteChange::New { key, route };

        let sim = SettlementSimulation::new(
            cx_with_bridges(&[Edge::new(v2(0, 1), v2(0, 2))]),
            Arc::new(()),
        );

        // When
        block_on(sim.update_route_to_gates(&[route_change]));

        // Then
        assert_eq!(
            *sim.cx.route_to_gates.lock().unwrap(),
            hashmap! { key => hashset!{ v2(0, 1), v2(0, 2) } }
        );
    }

    #[test]
    fn should_do_nothing_for_new_route_with_no_gates() {
        // Given
        let key = RouteKey {
            settlement: v2(0, 0),
            resource: Resource::Truffles,
            destination: v2(2, 2),
        };
        let route = Route {
            path: vec![v2(0, 0), v2(0, 1), v2(0, 2), v2(1, 2), v2(2, 2)],
            start_micros: 0,
            duration: Duration::from_secs(0),
            traffic: 0,
        };

        let route_change = RouteChange::New { key, route };

        let sim = SettlementSimulation::new(cx(), Arc::new(()));

        // When
        block_on(sim.update_route_to_gates(&[route_change]));

        // Then
        assert_eq!(*sim.cx.route_to_gates.lock().unwrap(), hashmap! {});
    }

    #[test]
    fn should_update_entry_for_updated_route_with_updated_path_with_gates() {
        // Given
        let key = RouteKey {
            settlement: v2(0, 0),
            resource: Resource::Truffles,
            destination: v2(2, 2),
        };
        let old = Route {
            path: vec![v2(0, 0), v2(1, 0), v2(2, 0), v2(2, 1), v2(2, 2)],
            start_micros: 0,
            duration: Duration::from_secs(0),
            traffic: 0,
        };
        let new = Route {
            path: vec![v2(0, 0), v2(0, 1), v2(0, 2), v2(1, 2), v2(2, 2)],
            start_micros: 0,
            duration: Duration::from_secs(0),
            traffic: 0,
        };

        let cx = cx_with_bridges(&[Edge::new(v2(0, 0), v2(0, 1)), Edge::new(v2(0, 0), v2(1, 0))]);
        *cx.route_to_gates.lock().unwrap() = hashmap! { key => hashset!{ v2(0, 0), v2(1, 0) } };

        let route_change = RouteChange::Updated { key, old, new };

        let sim = SettlementSimulation::new(cx, Arc::new(()));

        // When
        block_on(sim.update_route_to_gates(&[route_change]));

        // Then
        assert_eq!(
            *sim.cx.route_to_gates.lock().unwrap(),
            hashmap! { key => hashset!{ v2(0, 0), v2(0, 1) } }
        );
    }

    #[test]
    fn should_remove_entry_for_updated_route_with_no_gates() {
        // Given
        let key = RouteKey {
            settlement: v2(0, 0),
            resource: Resource::Truffles,
            destination: v2(2, 2),
        };
        let old = Route {
            path: vec![v2(0, 0), v2(1, 0), v2(2, 0), v2(2, 1), v2(2, 2)],
            start_micros: 0,
            duration: Duration::from_secs(0),
            traffic: 0,
        };
        let new = Route {
            path: vec![v2(0, 0), v2(0, 1), v2(0, 2), v2(1, 2), v2(2, 2)],
            start_micros: 0,
            duration: Duration::from_secs(0),
            traffic: 0,
        };

        let cx = cx_with_bridges(&[Edge::new(v2(0, 0), v2(1, 0))]);
        *cx.route_to_gates.lock().unwrap() = hashmap! { key => hashset!{ v2(0, 0), v2(1, 0) } };

        let route_change = RouteChange::Updated { key, old, new };

        let sim = SettlementSimulation::new(cx, Arc::new(()));

        // When
        block_on(sim.update_route_to_gates(&[route_change]));

        // Then
        assert_eq!(*sim.cx.route_to_gates.lock().unwrap(), hashmap! {});
    }

    #[test]
    fn should_do_nothing_for_updated_route_with_same_path() {
        // Given
        let key = RouteKey {
            settlement: v2(0, 0),
            resource: Resource::Truffles,
            destination: v2(2, 2),
        };
        let old = Route {
            path: vec![v2(0, 0), v2(0, 1), v2(0, 2), v2(1, 2), v2(2, 2)],
            start_micros: 0,
            duration: Duration::from_secs(0),
            traffic: 0,
        };
        let new = Route {
            path: vec![v2(0, 0), v2(0, 1), v2(0, 2), v2(1, 2), v2(2, 2)],
            start_micros: 10,
            duration: Duration::from_secs(0),
            traffic: 0,
        };

        let cx = cx_with_bridges(&[Edge::new(v2(0, 0), v2(0, 1))]);
        *cx.route_to_gates.lock().unwrap() = hashmap! {}; // Incorrect so we can check it is not corrected

        let route_change = RouteChange::Updated { key, old, new };

        let sim = SettlementSimulation::new(cx, Arc::new(()));

        // When
        block_on(sim.update_route_to_gates(&[route_change]));

        // Then
        assert_eq!(*sim.cx.route_to_gates.lock().unwrap(), hashmap! {});
    }

    #[test]
    fn should_remove_entry_for_removed_route() {
        // Given
        let key = RouteKey {
            settlement: v2(0, 0),
            resource: Resource::Truffles,
            destination: v2(2, 2),
        };
        let route = Route {
            path: vec![v2(0, 0), v2(0, 1), v2(0, 2), v2(1, 2), v2(2, 2)],
            start_micros: 0,
            duration: Duration::from_secs(0),
            traffic: 0,
        };

        let cx = cx();
        *cx.route_to_gates.lock().unwrap() = hashmap! { key => hashset!{ v2(0, 0), v2(0, 1) } };

        let route_change = RouteChange::Removed { key, route };

        let sim = SettlementSimulation::new(cx, Arc::new(()));

        // When
        block_on(sim.update_route_to_gates(&[route_change]));

        // Then
        assert_eq!(*sim.cx.route_to_gates.lock().unwrap(), hashmap! {});
    }

    #[test]
    fn multiple_changes() {
        // Given
        let key_new = RouteKey {
            settlement: v2(0, 0),
            resource: Resource::Truffles,
            destination: v2(2, 2),
        };
        let route_new = Route {
            path: vec![v2(0, 0), v2(0, 1), v2(0, 2), v2(1, 2), v2(2, 2)],
            start_micros: 0,
            duration: Duration::from_secs(0),
            traffic: 0,
        };
        let key_removed = RouteKey {
            settlement: v2(0, 0),
            resource: Resource::Truffles,
            destination: v2(1, 1),
        };
        let route_removed = Route {
            path: vec![v2(0, 0), v2(0, 1), v2(1, 1)],
            start_micros: 0,
            duration: Duration::from_secs(0),
            traffic: 0,
        };

        let cx = cx_with_bridges(&[Edge::new(v2(0, 0), v2(0, 1))]);
        *cx.route_to_gates.lock().unwrap() =
            hashmap! { key_removed => hashset!{ v2(0, 0), v2(0, 1) } };

        let route_changes = vec![
            RouteChange::New {
                key: key_new,
                route: route_new,
            },
            RouteChange::Removed {
                key: key_removed,
                route: route_removed,
            },
        ];

        let sim = SettlementSimulation::new(cx, Arc::new(()));

        // When
        block_on(sim.update_route_to_gates(&route_changes));

        // Then
        assert_eq!(
            *sim.cx.route_to_gates.lock().unwrap(),
            hashmap! { key_new => hashset!{ v2(0, 0), v2(0, 1) } }
        );
    }
}
