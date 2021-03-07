use crate::simulation::settlement::model::RouteChange;
use crate::simulation::settlement::SettlementSimulation;
use crate::traits::RefreshPositions;
use commons::V2;
use std::collections::HashSet;

impl<T> SettlementSimulation<T>
where
    T: RefreshPositions,
{
    pub async fn refresh_positions(&self, route_changes: &[RouteChange]) {
        let to_refresh = get_all_positions_to_refresh(route_changes);
        self.cx.refresh_positions(to_refresh).await;
    }
}

fn get_all_positions_to_refresh(route_changes: &[RouteChange]) -> HashSet<V2<usize>> {
    route_changes
        .iter()
        .flat_map(|route_change| get_positions_to_refresh(route_change))
        .collect()
}

fn get_positions_to_refresh<'a>(
    route_change: &'a RouteChange,
) -> Box<dyn Iterator<Item = V2<usize>> + 'a> {
    match route_change {
        RouteChange::New { route, .. } => Box::new(route.path.iter().copied()),
        RouteChange::Updated { old, new, .. } => {
            Box::new(new.path.iter().copied().chain(old.path.iter().copied()))
        }
        RouteChange::Removed { route, .. } => Box::new(route.path.iter().copied()),
        RouteChange::NoChange { route, .. } => Box::new(route.path.iter().copied()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::resource::Resource;
    use crate::route::{Route, RouteKey};
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
            path: vec![v2(1, 3), v2(1, 4), v2(1, 5)],
            start_micros: 0,
            duration: Duration::from_secs(2),
            traffic: 3,
        }
    }

    struct Cx {
        refreshed_positions: Mutex<HashSet<V2<usize>>>,
    }

    impl Default for Cx {
        fn default() -> Self {
            Cx {
                refreshed_positions: Mutex::default(),
            }
        }
    }

    #[async_trait]
    impl RefreshPositions for Cx {
        async fn refresh_positions(&self, positions: HashSet<V2<usize>>) {
            self.refreshed_positions
                .lock()
                .unwrap()
                .extend(&mut positions.into_iter())
        }
    }

    #[test]
    fn new_route_should_refresh_all_positions_in_route() {
        // Given
        let change = RouteChange::New {
            key: key(),
            route: route_1(),
        };
        let sim = SettlementSimulation::new(Cx::default());

        // When
        block_on(sim.refresh_positions(&[change]));

        // Then
        assert_eq!(
            *sim.cx.refreshed_positions.lock().unwrap(),
            hashset! {v2(1, 3), v2(2, 3), v2(2, 4), v2(2, 5), v2(1, 5)}
        );
    }

    #[test]
    fn updated_route_should_refresh_positions_from_old_and_new_route() {
        // Given
        let change = RouteChange::Updated {
            key: key(),
            old: route_1(),
            new: route_2(),
        };
        let sim = SettlementSimulation::new(Cx::default());

        // When
        block_on(sim.refresh_positions(&[change]));

        // Then
        assert_eq!(
            *sim.cx.refreshed_positions.lock().unwrap(),
            hashset! {v2(1, 3), v2(2, 3), v2(2, 4), v2(2, 5), v2(1, 5), v2(1, 4)}
        );
    }

    #[test]
    fn removed_route_should_refresh_all_positions_in_route() {
        // Given
        let change = RouteChange::Removed {
            key: key(),
            route: route_1(),
        };
        let sim = SettlementSimulation::new(Cx::default());

        // When
        block_on(sim.refresh_positions(&[change]));

        // Then
        assert_eq!(
            *sim.cx.refreshed_positions.lock().unwrap(),
            hashset! {v2(1, 3), v2(2, 3), v2(2, 4), v2(2, 5), v2(1, 5)}
        );
    }

    #[test]
    fn no_change_route_should_refresh_all_positions_in_route() {
        // Given
        let change = RouteChange::NoChange {
            key: key(),
            route: route_1(),
        };
        let sim = SettlementSimulation::new(Cx::default());

        // When
        block_on(sim.refresh_positions(&[change]));

        // Then
        assert_eq!(
            *sim.cx.refreshed_positions.lock().unwrap(),
            hashset! {v2(1, 3), v2(2, 3), v2(2, 4), v2(2, 5), v2(1, 5)}
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
        block_on(sim.refresh_positions(&[change_1, change_2]));

        // Then
        assert_eq!(
            *sim.cx.refreshed_positions.lock().unwrap(),
            hashset! {v2(1, 3), v2(1, 4), v2(2, 3), v2(2, 4), v2(2, 5), v2(1, 5)}
        );
    }
}
