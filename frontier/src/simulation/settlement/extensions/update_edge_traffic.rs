use crate::route::{Route, RouteKey};
use crate::simulation::settlement::model::RouteChange;
use crate::simulation::settlement::SettlementSimulation;
use crate::traffic::EdgeTraffic;
use crate::traits::{RefreshEdges, WithEdgeTraffic};
use commons::edge::{Edge, Edges};
use std::collections::hash_map::Entry;
use std::collections::HashSet;

impl<T> SettlementSimulation<T>
where
    T: RefreshEdges + WithEdgeTraffic,
{
    pub async fn update_edge_traffic(&self, route_changes: &[RouteChange]) {
        let changed_edges = self
            .update_all_edge_traffic_and_get_changes(route_changes)
            .await;
        self.cx.refresh_edges(changed_edges).await;
    }

    async fn update_all_edge_traffic_and_get_changes(
        &self,
        route_changes: &[RouteChange],
    ) -> HashSet<Edge> {
        self.cx
            .mut_edge_traffic(|edge_traffic| {
                update_all_edge_traffic_and_get_changes(edge_traffic, route_changes)
            })
            .await
    }
}

pub fn update_all_edge_traffic_and_get_changes(
    edge_traffic: &mut EdgeTraffic,
    route_changes: &[RouteChange],
) -> HashSet<Edge> {
    route_changes
        .iter()
        .flat_map(|route_change| update_edge_traffic_and_get_changes(edge_traffic, route_change))
        .collect()
}

fn update_edge_traffic_and_get_changes(
    edge_traffic: &mut EdgeTraffic,
    route_change: &RouteChange,
) -> Vec<Edge> {
    match route_change {
        RouteChange::New { key, route } => new(edge_traffic, &key, &route),
        RouteChange::Updated { key, old, new } => updated(edge_traffic, &key, &old, &new),
        RouteChange::Removed { key, route } => removed(edge_traffic, &key, &route),
        RouteChange::NoChange { route, .. } => no_change(&route),
    }
}

fn new(edge_traffic: &mut EdgeTraffic, key: &RouteKey, route: &Route) -> Vec<Edge> {
    let mut out = vec![];
    for edge in route.path.edges() {
        edge_traffic
            .entry(edge)
            .or_insert_with(HashSet::new)
            .insert(*key);
        out.push(edge);
    }
    out
}

fn updated(edge_traffic: &mut EdgeTraffic, key: &RouteKey, old: &Route, new: &Route) -> Vec<Edge> {
    let mut out = vec![];
    let old_edges: HashSet<Edge> = old.path.edges().collect();
    let new_edges: HashSet<Edge> = new.path.edges().collect();

    let added = new_edges.difference(&old_edges).cloned();
    let removed = old_edges.difference(&new_edges).cloned();
    let union = new_edges.union(&old_edges).cloned();

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

    for edge in union {
        out.push(edge);
    }

    out
}

fn removed(edge_traffic: &mut EdgeTraffic, key: &RouteKey, route: &Route) -> Vec<Edge> {
    let mut out = vec![];
    for edge in route.path.edges() {
        if let Entry::Occupied(mut entry) = edge_traffic.entry(edge) {
            entry.get_mut().remove(key);
            if entry.get().is_empty() {
                entry.remove_entry();
            }
        }
        out.push(edge);
    }
    out
}

fn no_change(route: &Route) -> Vec<Edge> {
    route.path.edges().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::resource::Resource;
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
        refreshed_edges: Mutex<HashSet<Edge>>,
    }

    #[async_trait]
    impl RefreshEdges for Cx {
        async fn refresh_edges(&self, edges: HashSet<Edge>) {
            self.refreshed_edges
                .lock()
                .unwrap()
                .extend(&mut edges.into_iter());
        }
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
    fn new_route_should_refresh_all_edges_in_route() {
        // Given
        let change = RouteChange::New {
            key: key(),
            route: route_1(),
        };
        let sim = SettlementSimulation::new(Cx::default());

        // When
        block_on(sim.update_edge_traffic(&[change]));

        // Then
        assert_eq!(
            *sim.cx.refreshed_edges.lock().unwrap(),
            hashset! {
                Edge::new(v2(1, 3), v2(2, 3)),
                Edge::new(v2(2, 3), v2(2, 4)),
                Edge::new(v2(2, 4), v2(2, 5)),
                Edge::new(v2(2, 5), v2(1, 5)),
            },
        );
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
        block_on(sim.update_edge_traffic(&[change]));

        // Then
        let mut expected = hashmap! {};
        for edge in route_1().path.edges() {
            expected.insert(edge, hashset! {key()});
        }
        assert_eq!(*sim.cx.edge_traffic.lock().unwrap(), expected);
    }

    #[test]
    fn updated_route_should_refresh_edges_in_old_and_new() {
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
            ..Cx::default()
        };
        let sim = SettlementSimulation::new(cx);

        // When
        block_on(sim.update_edge_traffic(&[change]));

        // Then
        assert_eq!(
            *sim.cx.refreshed_edges.lock().unwrap(),
            hashset! {
             Edge::new(v2(1, 3), v2(2, 3)),
             Edge::new(v2(2, 3), v2(2, 4)),
             Edge::new(v2(2, 4), v2(2, 5)),
             Edge::new(v2(2, 5), v2(1, 5)),
             Edge::new(v2(1, 3), v2(1, 4)),
             Edge::new(v2(1, 4), v2(2, 4)),
            }
        );
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
            ..Cx::default()
        };
        let sim = SettlementSimulation::new(cx);

        // When
        block_on(sim.update_edge_traffic(&[change]));

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
            ..Cx::default()
        };
        let sim = SettlementSimulation::new(cx);

        // When
        block_on(sim.update_edge_traffic(&[change]));

        // Then
        let mut expected = hashmap! {};
        for edge in route_1().path.edges() {
            expected.insert(edge, hashset! {key()});
        }
        assert_eq!(*sim.cx.edge_traffic.lock().unwrap(), expected);
    }

    #[test]
    fn removed_route_should_refresh_all_edges_in_route() {
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
            ..Cx::default()
        };
        let sim = SettlementSimulation::new(cx);

        // When
        block_on(sim.update_edge_traffic(&[change]));

        // Then
        assert_eq!(
            *sim.cx.refreshed_edges.lock().unwrap(),
            hashset! {
             Edge::new(v2(1, 3), v2(2, 3)),
             Edge::new(v2(2, 3), v2(2, 4)),
             Edge::new(v2(2, 4), v2(2, 5)),
             Edge::new(v2(2, 5), v2(1, 5)),
            }
        );
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
            ..Cx::default()
        };
        let sim = SettlementSimulation::new(cx);

        // When
        block_on(sim.update_edge_traffic(&[change]));

        // Then
        let expected = hashmap! {};
        assert_eq!(*sim.cx.edge_traffic.lock().unwrap(), expected);
    }

    #[test]
    fn no_change_route_should_refresh_all_edges_in_route() {
        // Given
        let change = RouteChange::NoChange {
            key: key(),
            route: route_1(),
        };
        let sim = SettlementSimulation::new(Cx::default());

        // When
        block_on(sim.update_edge_traffic(&[change]));

        // Then
        assert_eq!(
            *sim.cx.refreshed_edges.lock().unwrap(),
            hashset! {
             Edge::new(v2(1, 3), v2(2, 3)),
             Edge::new(v2(2, 3), v2(2, 4)),
             Edge::new(v2(2, 4), v2(2, 5)),
             Edge::new(v2(2, 5), v2(1, 5)),
            }
        );
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
        block_on(sim.update_edge_traffic(&[change]));

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
            ..Cx::default()
        };
        let sim = SettlementSimulation::new(cx);

        // When
        block_on(sim.update_edge_traffic(&[change]));

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
            ..Cx::default()
        };
        let sim = SettlementSimulation::new(cx);

        // When
        block_on(sim.update_edge_traffic(&[change]));

        // Then
        let mut expected = hashmap! {};
        for edge in route_1().path.edges() {
            expected.insert(edge, hashset! {key_2});
        }
        assert_eq!(*sim.cx.edge_traffic.lock().unwrap(), expected);
    }

    #[test]
    fn multiple_changes() {
        // Given
        let change_1 = RouteChange::New {
            key: key(),
            route: route_1(),
        };
        let change_2 = RouteChange::New {
            key: RouteKey {
                settlement: v2(1, 3),
                resource: Resource::Coal,
                destination: v2(1, 5),
            },
            route: route_2(),
        };
        let sim = SettlementSimulation::new(Cx::default());

        // When
        block_on(sim.update_edge_traffic(&[change_1, change_2]));

        // Then
        assert_eq!(
            *sim.cx.refreshed_edges.lock().unwrap(),
            hashset! {
               Edge::new(v2(1, 3), v2(2, 3)),
               Edge::new(v2(2, 3), v2(2, 4)),
               Edge::new(v2(2, 4), v2(2, 5)),
               Edge::new(v2(2, 5), v2(1, 5)),
               Edge::new(v2(1, 3), v2(1, 4)),
               Edge::new(v2(1, 4), v2(2, 4)),
            }
        );
    }
}
