use crate::avatar::CheckForPort;
use crate::route::RouteKey;
use crate::simulation::settlement::model::RouteChange;
use crate::simulation::settlement::SettlementSimulation;
use crate::traits::{WithRouteToPorts, WithWorld};
use crate::world::World;
use commons::edge::Edges;
use commons::V2;
use std::collections::{HashMap, HashSet};

impl<T> SettlementSimulation<T>
where
    T: WithRouteToPorts + WithWorld,
{
    pub async fn update_route_to_ports<C>(&self, route_changes: &[RouteChange], port_checker: &C)
    where
        C: CheckForPort + Clone + Send + Sync + 'static,
    {
        let (_, updated) = join!(self.remove_removed(route_changes), async {
            get_all_updated(route_changes)
        });
        let ports = self.get_all_ports(&updated, port_checker).await;
        self.update_ports(ports).await;
    }

    async fn remove_removed(&self, route_changes: &[RouteChange]) {
        let removed = get_all_removed(route_changes);
        self.tx
            .mut_route_to_ports(|route_to_ports| {
                for key in removed {
                    route_to_ports.remove(key);
                }
            })
            .await;
    }

    async fn get_all_ports<C>(
        &self,
        routes: &HashMap<RouteKey, Vec<V2<usize>>>,
        port_checker: &C,
    ) -> HashMap<RouteKey, HashSet<V2<usize>>>
    where
        C: CheckForPort + Clone + Send + Sync + 'static,
    {
        self.tx
            .with_world(|world| get_all_ports(world, port_checker, routes))
            .await
    }

    async fn update_ports(&self, ports: HashMap<RouteKey, HashSet<V2<usize>>>) {
        self.tx
            .mut_route_to_ports(|route_to_ports| {
                for (key, ports) in ports {
                    if ports.is_empty() {
                        route_to_ports.remove(&key);
                    } else {
                        route_to_ports.insert(key, ports);
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

fn get_all_ports(
    world: &World,
    port_checker: &dyn CheckForPort,
    routes: &HashMap<RouteKey, Vec<V2<usize>>>,
) -> HashMap<RouteKey, HashSet<V2<usize>>> {
    routes
        .iter()
        .map(|(key, path)| (*key, get_ports(world, port_checker, &path)))
        .collect()
}

fn get_ports(
    world: &World,
    port_checker: &dyn CheckForPort,
    path: &[V2<usize>],
) -> HashSet<V2<usize>> {
    path.edges()
        .flat_map(|edge| port_checker.check_for_port(world, edge.from(), edge.to()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resource::Resource;
    use crate::route::Route;
    use crate::world::World;
    use commons::async_trait::async_trait;
    use commons::{v2, M};
    use futures::executor::block_on;
    use std::sync::Mutex;
    use std::time::Duration;

    struct Tx {
        route_to_ports: Mutex<HashMap<RouteKey, HashSet<V2<usize>>>>,
        world: Mutex<World>,
    }

    #[async_trait]
    impl WithWorld for Tx {
        async fn with_world<F, O>(&self, function: F) -> O
        where
            F: FnOnce(&World) -> O + Send,
        {
            function(&self.world.lock().unwrap())
        }

        async fn mut_world<F, O>(&self, function: F) -> O
        where
            F: FnOnce(&mut World) -> O + Send,
        {
            function(&mut self.world.lock().unwrap())
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

    fn tx() -> Tx {
        Tx {
            route_to_ports: Mutex::default(),
            world: Mutex::new(World::new(M::zeros(3, 3), 0.0)),
        }
    }

    impl CheckForPort for HashSet<V2<usize>> {
        fn check_for_port(&self, _: &World, from: &V2<usize>, _: &V2<usize>) -> Option<V2<usize>> {
            if self.contains(from) {
                Some(*from)
            } else {
                None
            }
        }
    }

    #[test]
    fn should_insert_entry_for_new_route_with_ports() {
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

        let sim = SettlementSimulation::new(tx());

        // When
        block_on(sim.update_route_to_ports(&[route_change], &hashset! {v2(0, 1), v2(1, 2)}));

        // Then
        assert_eq!(
            *sim.tx.route_to_ports.lock().unwrap(),
            hashmap! { key => hashset!{ v2(0, 1), v2(1, 2) } }
        );
    }

    #[test]
    fn should_do_nothing_for_new_route_with_no_ports() {
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

        let sim = SettlementSimulation::new(tx());

        // When
        block_on(sim.update_route_to_ports(&[route_change], &hashset! {}));

        // Then
        assert_eq!(*sim.tx.route_to_ports.lock().unwrap(), hashmap! {});
    }

    #[test]
    fn should_update_entry_for_updated_route_with_updated_path_with_ports() {
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

        let tx = tx();
        *tx.route_to_ports.lock().unwrap() = hashmap! { key => hashset!{ v2(1, 0) } };

        let route_change = RouteChange::Updated { key, old, new };

        let sim = SettlementSimulation::new(tx);

        // When
        block_on(
            sim.update_route_to_ports(&[route_change], &hashset! {v2(0, 1), v2(1, 0), v2(1, 2)}),
        );

        // Then
        assert_eq!(
            *sim.tx.route_to_ports.lock().unwrap(),
            hashmap! { key => hashset!{ v2(0, 1), v2(1, 2) } }
        );
    }

    #[test]
    fn should_remove_entry_for_updated_route_with_no_ports() {
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

        let tx = tx();
        *tx.route_to_ports.lock().unwrap() = hashmap! { key => hashset!{ v2(1, 0) } };

        let route_change = RouteChange::Updated { key, old, new };

        let sim = SettlementSimulation::new(tx);

        // When
        block_on(sim.update_route_to_ports(&[route_change], &hashset! {v2(1, 0)}));

        // Then
        assert_eq!(*sim.tx.route_to_ports.lock().unwrap(), hashmap! {});
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

        let tx = tx();
        *tx.route_to_ports.lock().unwrap() = hashmap! {}; // Incorrect so we can check it is not corrected

        let route_change = RouteChange::Updated { key, old, new };

        let sim = SettlementSimulation::new(tx);

        // When
        block_on(
            sim.update_route_to_ports(&[route_change], &hashset! {v2(0, 1), v2(1, 0), v2(1, 2)}),
        );

        // Then
        assert_eq!(*sim.tx.route_to_ports.lock().unwrap(), hashmap! {});
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

        let tx = tx();
        *tx.route_to_ports.lock().unwrap() = hashmap! { key => hashset!{ v2(0, 1), v2(1, 2) } };

        let route_change = RouteChange::Removed { key, route };

        let sim = SettlementSimulation::new(tx);

        // When
        block_on(sim.update_route_to_ports(&[route_change], &hashset! {}));

        // Then
        assert_eq!(*sim.tx.route_to_ports.lock().unwrap(), hashmap! {});
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

        let tx = tx();
        *tx.route_to_ports.lock().unwrap() = hashmap! { key_removed => hashset!{ v2(0, 1) } };

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

        let sim = SettlementSimulation::new(tx);

        // When
        block_on(sim.update_route_to_ports(&route_changes, &hashset! {v2(0, 1), v2(1, 2)}));

        // Then
        assert_eq!(
            *sim.tx.route_to_ports.lock().unwrap(),
            hashmap! { key_new => hashset!{ v2(0, 1), v2(1, 2) } }
        );
    }
}
