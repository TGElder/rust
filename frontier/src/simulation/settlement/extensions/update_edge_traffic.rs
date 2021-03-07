use crate::route::{Route, RouteKey};
use crate::simulation::settlement::model::RouteChange;
use crate::simulation::settlement::SettlementSimulation;
use crate::traits::WithEdgeTraffic;
use commons::edge::{Edge, Edges};
use futures::future::join_all;
use std::collections::hash_map::Entry;
use std::collections::HashSet;

impl<T> SettlementSimulation<T>
where
    T: WithEdgeTraffic,
{
    pub async fn update_all_edge_traffic(&self, route_changes: &[RouteChange]) {
        join_all(
            route_changes
                .iter()
                .map(|route_change| self.update_edge_traffic(route_change)),
        )
        .await;
    }

    async fn update_edge_traffic(&self, route_change: &RouteChange) {
        match route_change {
            RouteChange::New { key, route } => self.new_edge_traffic(&key, &route).await,
            RouteChange::Updated { key, old, new } => {
                self.updated_edge_traffic(&key, &old, &new).await
            }
            RouteChange::Removed { key, route } => self.removed_edge_traffic(&key, &route).await,
            _ => (),
        }
    }

    async fn new_edge_traffic(&self, key: &RouteKey, route: &Route) {
        self.cx
            .mut_edge_traffic(|edge_traffic| {
                for edge in route.path.edges() {
                    edge_traffic
                        .entry(edge)
                        .or_insert_with(HashSet::new)
                        .insert(*key);
                }
            })
            .await;
    }

    async fn updated_edge_traffic(&self, key: &RouteKey, old: &Route, new: &Route) {
        let old_edges: HashSet<Edge> = old.path.edges().collect();
        let new_edges: HashSet<Edge> = new.path.edges().collect();

        let added = new_edges.difference(&old_edges).cloned();
        let removed = old_edges.difference(&new_edges).cloned();

        self.cx
            .mut_edge_traffic(|edge_traffic| {
                for edge in added {
                    edge_traffic
                        .entry(edge)
                        .or_insert_with(HashSet::new)
                        .insert(*key);
                }

                for edge in removed {
                    if let Entry::Occupied(mut entry) = edge_traffic.entry(edge) {
                        entry.get_mut().remove(key);
                        if entry.get().is_empty() {
                            entry.remove_entry();
                        }
                    }
                }
            })
            .await;
    }

    async fn removed_edge_traffic(&self, key: &RouteKey, route: &Route) {
        self.cx
            .mut_edge_traffic(|edge_traffic| {
                for edge in route.path.edges() {
                    if let Entry::Occupied(mut entry) = edge_traffic.entry(edge) {
                        entry.get_mut().remove(key);
                        if entry.get().is_empty() {
                            entry.remove_entry();
                        }
                    }
                }
            })
            .await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::resource::Resource;
    use crate::traffic::EdgeTraffic;
    use commons::async_trait::async_trait;
    use commons::v2;
    use futures::executor::block_on;
    use std::sync::Mutex;
    use std::time::Duration;

    fn key() -> RouteKey {
        RouteKey {
            settlement: v2(1, 3),
            resource: Resource::Coal,
            destination: v2(1, 5),
        }
    }

    fn route_1() -> Route {
        Route {
            path: vec![v2(1, 3), v2(2, 3), v2(2, 4), v2(2, 5), v2(1, 5)],
            start_micros: 0,
            duration: Duration::from_secs(4),
            traffic: 3,
        }
    }

    fn route_2() -> Route {
        Route {
            path: vec![v2(1, 3), v2(1, 4), v2(2, 4), v2(2, 5), v2(1, 5)],
            start_micros: 0,
            duration: Duration::from_secs(2),
            traffic: 3,
        }
    }

    #[derive(Default)]
    struct Cx {
        edge_traffic: Mutex<EdgeTraffic>,
    }

    #[async_trait]
    impl WithEdgeTraffic for Cx {
        async fn with_edge_traffic<F, O>(&self, function: F) -> O
        where
            F: FnOnce(&EdgeTraffic) -> O + Send,
        {
            function(&self.edge_traffic.lock().unwrap())
        }

        async fn mut_edge_traffic<F, O>(&self, function: F) -> O
        where
            F: FnOnce(&mut EdgeTraffic) -> O + Send,
        {
            function(&mut self.edge_traffic.lock().unwrap())
        }
    }

    #[test]
    fn new_route_should_add_edge_traffic_for_all_edges_in_route() {
        // Given
        let change = RouteChange::New {
            key: key(),
            route: route_1(),
        };
        let cx = Cx::default();
        let sim = SettlementSimulation::new(cx);

        // When
        block_on(sim.update_all_edge_traffic(&[change]));

        // Then
        let mut expected = hashmap! {};
        for edge in route_1().path.edges() {
            expected.insert(edge, hashset! {key()});
        }
        assert_eq!(*sim.cx.edge_traffic.lock().unwrap(), expected);
    }

    #[test]
    fn updated_route_should_remove_edge_traffic_for_edges_not_in_new_route() {
        // Given
        let change = RouteChange::Updated {
            key: key(),
            old: route_1(),
            new: route_2(),
        };
        let mut edge_traffic = EdgeTraffic::default();
        for edge in route_1().path.edges() {
            edge_traffic.insert(edge, hashset! {key()});
        }
        let cx = Cx {
            edge_traffic: Mutex::new(edge_traffic),
        };
        let sim = SettlementSimulation::new(cx);

        // When
        block_on(sim.update_all_edge_traffic(&[change]));

        // Then
        let mut expected = hashmap! {};
        for edge in route_2().path.edges() {
            expected.insert(edge, hashset! {key()});
        }
        assert_eq!(*sim.cx.edge_traffic.lock().unwrap(), expected);
    }

    #[test]
    fn updated_route_should_add_edge_traffic_for_edges_not_in_old_route() {
        // Given
        let change = RouteChange::Updated {
            key: key(),
            old: route_2(),
            new: route_1(),
        };
        let mut edge_traffic = EdgeTraffic::default();
        for edge in route_2().path.edges() {
            edge_traffic.insert(edge, hashset! {key()});
        }
        let cx = Cx {
            edge_traffic: Mutex::new(edge_traffic),
        };
        let sim = SettlementSimulation::new(cx);

        // When
        block_on(sim.update_all_edge_traffic(&[change]));

        // Then
        let mut expected = hashmap! {};
        for edge in route_1().path.edges() {
            expected.insert(edge, hashset! {key()});
        }
        assert_eq!(*sim.cx.edge_traffic.lock().unwrap(), expected);
    }

    #[test]
    fn removed_route_should_remove_edge_traffic_for_all_edges_in_route() {
        // Given
        let change = RouteChange::Removed {
            key: key(),
            route: route_1(),
        };
        let mut edge_traffic = EdgeTraffic::default();
        for edge in route_1().path.edges() {
            edge_traffic.insert(edge, hashset! {key()});
        }
        let cx = Cx {
            edge_traffic: Mutex::new(edge_traffic),
        };
        let sim = SettlementSimulation::new(cx);

        // When
        block_on(sim.update_all_edge_traffic(&[change]));

        // Then
        let expected = hashmap! {};
        assert_eq!(*sim.cx.edge_traffic.lock().unwrap(), expected);
    }

    #[test]
    fn no_change_route_should_not_change_edge_traffic() {
        // Given
        let change = RouteChange::NoChange {
            key: key(),
            route: route_1(),
        };
        let sim = SettlementSimulation::new(Cx::default());

        // When
        block_on(sim.update_all_edge_traffic(&[change]));

        // Then
        assert_eq!(*sim.cx.edge_traffic.lock().unwrap(), hashmap! {},);
    }

    #[test]
    fn should_retain_traffic_added_by_other_route_when_adding_route() {
        // Given
        let change = RouteChange::New {
            key: key(),
            route: route_1(),
        };
        let key_2 = RouteKey {
            settlement: v2(1, 4),
            resource: Resource::Coal,
            destination: v2(1, 5),
        };
        let mut edge_traffic = EdgeTraffic::default();
        for edge in route_1().path.edges() {
            edge_traffic.insert(edge, hashset! {key_2});
        }
        let cx = Cx {
            edge_traffic: Mutex::new(edge_traffic),
        };
        let sim = SettlementSimulation::new(cx);

        // When
        block_on(sim.update_all_edge_traffic(&[change]));

        // Then
        let mut expected = hashmap! {};
        for edge in route_1().path.edges() {
            expected.insert(edge, hashset! {key_2, key()});
        }
        assert_eq!(*sim.cx.edge_traffic.lock().unwrap(), expected);
    }

    #[test]
    fn should_retain_traffic_added_by_other_route_when_removing_route() {
        // Given
        let change = RouteChange::Removed {
            key: key(),
            route: route_1(),
        };
        let key_2 = RouteKey {
            settlement: v2(1, 4),
            resource: Resource::Coal,
            destination: v2(1, 5),
        };
        let mut edge_traffic = EdgeTraffic::default();
        for edge in route_1().path.edges() {
            edge_traffic.insert(edge, hashset! {key_2, key()});
        }
        let cx = Cx {
            edge_traffic: Mutex::new(edge_traffic),
        };
        let sim = SettlementSimulation::new(cx);

        // When
        block_on(sim.update_all_edge_traffic(&[change]));

        // Then
        let mut expected = hashmap! {};
        for edge in route_1().path.edges() {
            expected.insert(edge, hashset! {key_2});
        }
        assert_eq!(*sim.cx.edge_traffic.lock().unwrap(), expected);
    }
}
