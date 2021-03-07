use crate::simulation::settlement::model::RouteChange;
use crate::simulation::settlement::SettlementSimulation;
use crate::traits::RefreshEdges;
use commons::edge::{Edge, Edges};
use std::collections::HashSet;

impl<T> SettlementSimulation<T>
where
    T: RefreshEdges,
{
    pub async fn refresh_edges(&self, route_changes: &[RouteChange]) {
        let to_refresh = get_all_edges_to_refresh(route_changes);
        self.cx.refresh_edges(to_refresh).await;
    }
}

fn get_all_edges_to_refresh(route_changes: &[RouteChange]) -> HashSet<Edge> {
    route_changes
        .iter()
        .flat_map(|route_change| get_edges_to_refresh(route_change))
        .collect()
}

fn get_edges_to_refresh<'a>(route_change: &'a RouteChange) -> Box<dyn Iterator<Item = Edge> + 'a> {
    match route_change {
        RouteChange::New { route, .. } => Box::new(route.path.edges()),
        RouteChange::Updated { old, new, .. } => Box::new(new.path.edges().chain(old.path.edges())),
        RouteChange::Removed { route, .. } => Box::new(route.path.edges()),
        RouteChange::NoChange { route, .. } => Box::new(route.path.edges()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::resource::Resource;
    use crate::route::{Route, RouteKey};
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

    #[test]
    fn new_route_should_refresh_all_edges_in_route() {
        // Given
        let change = RouteChange::New {
            key: key(),
            route: route_1(),
        };
        let sim = SettlementSimulation::new(Cx::default());

        // When
        block_on(sim.refresh_edges(&[change]));

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
        let sim = SettlementSimulation::new(Cx::default());

        // When
        block_on(sim.refresh_edges(&[change]));

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
        let sim = SettlementSimulation::new(Cx::default());

        // When
        block_on(sim.refresh_edges(&[change]));

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
    fn no_change_route_should_refresh_all_edges_in_route() {
        // Given
        let change = RouteChange::NoChange {
            key: key(),
            route: route_1(),
        };
        let sim = SettlementSimulation::new(Cx::default());

        // When
        block_on(sim.refresh_edges(&[change]));

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
        block_on(sim.refresh_edges(&[change_1, change_2]));

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
