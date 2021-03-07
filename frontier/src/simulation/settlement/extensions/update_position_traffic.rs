use crate::route::{Route, RouteKey};
use crate::simulation::settlement::model::RouteChange;
use crate::simulation::settlement::SettlementSimulation;
use crate::traits::{RefreshPositions, WithTraffic};
use commons::grid::Grid;
use commons::V2;
use std::collections::HashSet;

impl<T> SettlementSimulation<T>
where
    T: RefreshPositions + WithTraffic,
{
    pub async fn update_position_traffic(&self, route_changes: &[RouteChange]) {
        let changed_positions = self
            .update_all_position_traffic_and_get_changes(route_changes)
            .await;
        self.cx.refresh_positions(changed_positions).await;
    }

    async fn update_all_position_traffic_and_get_changes(
        &self,
        route_changes: &[RouteChange],
    ) -> HashSet<V2<usize>> {
        let mut out = HashSet::new();
        for route_change in route_changes {
            out.extend(
                &mut self
                    .update_position_traffic_and_get_changes(route_change)
                    .await
                    .into_iter(),
            );
        }
        out
    }

    async fn update_position_traffic_and_get_changes(
        &self,
        route_change: &RouteChange,
    ) -> Vec<V2<usize>> {
        match route_change {
            RouteChange::New { key, route } => self.new_position_traffic(&key, &route).await,
            RouteChange::Updated { key, old, new } => {
                self.updated_position_traffic(&key, &old, &new).await
            }
            RouteChange::Removed { key, route } => self.remove_position_traffic(&key, &route).await,
            RouteChange::NoChange { route, .. } => no_change(&route),
        }
    }

    async fn new_position_traffic(&self, key: &RouteKey, route: &Route) -> Vec<V2<usize>> {
        let mut out = vec![];
        self.cx
            .mut_traffic(|traffic| {
                for position in route.path.iter() {
                    traffic.mut_cell_unsafe(&position).insert(*key);
                    out.push(*position);
                }
            })
            .await;
        out
    }

    async fn updated_position_traffic(
        &self,
        key: &RouteKey,
        old: &Route,
        new: &Route,
    ) -> Vec<V2<usize>> {
        let mut out = vec![];

        let old: HashSet<&V2<usize>> = old.path.iter().collect();
        let new: HashSet<&V2<usize>> = new.path.iter().collect();

        let added = new.difference(&old).cloned();
        let removed = old.difference(&new).cloned();
        let union = new.union(&old).cloned();

        self.cx
            .mut_traffic(|traffic| {
                for position in added {
                    traffic.mut_cell_unsafe(&position).insert(*key);
                }

                for position in removed {
                    traffic.mut_cell_unsafe(&position).remove(key);
                }

                for position in union {
                    out.push(*position);
                }
            })
            .await;

        out
    }

    async fn remove_position_traffic(&self, key: &RouteKey, route: &Route) -> Vec<V2<usize>> {
        let mut out = vec![];
        self.cx
            .mut_traffic(|traffic| {
                for position in route.path.iter() {
                    traffic.mut_cell_unsafe(&position).remove(key);
                    out.push(*position);
                }
            })
            .await;

        out
    }
}

fn no_change(route: &Route) -> Vec<V2<usize>> {
    route.path.clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::resource::Resource;
    use crate::traffic::Traffic;
    use commons::async_trait::async_trait;
    use commons::index2d::Vec2D;
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

    fn traffic() -> Traffic {
        Vec2D::new(6, 6, HashSet::with_capacity(0))
    }

    struct Cx {
        refreshed_positions: Mutex<HashSet<V2<usize>>>,
        traffic: Mutex<Traffic>,
    }

    impl Default for Cx {
        fn default() -> Self {
            Cx {
                refreshed_positions: Mutex::default(),
                traffic: Mutex::new(traffic()),
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

    #[async_trait]
    impl WithTraffic for Cx {
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
    fn new_route_should_refresh_all_positions_in_route() {
        // Given
        let change = RouteChange::New {
            key: key(),
            route: route_1(),
        };
        let sim = SettlementSimulation::new(Cx::default());

        // When
        block_on(sim.update_position_traffic(&[change]));

        // Then
        assert_eq!(
            *sim.cx.refreshed_positions.lock().unwrap(),
            hashset! {v2(1, 3), v2(2, 3), v2(2, 4), v2(2, 5), v2(1, 5)}
        );
    }

    #[test]
    fn new_route_should_add_traffic_for_all_positions_in_route() {
        // Given
        let change = RouteChange::New {
            key: key(),
            route: route_1(),
        };
        let sim = SettlementSimulation::new(Cx::default());

        // When
        block_on(sim.update_position_traffic(&[change]));

        // Then
        let mut expected = traffic();
        for position in route_1().path.iter() {
            expected.mut_cell_unsafe(position).insert(key());
        }
        assert_eq!(*sim.cx.traffic.lock().unwrap(), expected);
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
        block_on(sim.update_position_traffic(&[change]));

        // Then
        assert_eq!(
            *sim.cx.refreshed_positions.lock().unwrap(),
            hashset! {v2(1, 3), v2(2, 3), v2(2, 4), v2(2, 5), v2(1, 5), v2(1, 4)}
        );
    }

    #[test]
    fn updated_route_should_remove_traffic_for_positions_not_in_new_route() {
        // Given
        let change = RouteChange::Updated {
            key: key(),
            old: route_1(),
            new: route_2(),
        };
        let mut tx_traffic = traffic();
        for position in route_1().path.iter() {
            tx_traffic.mut_cell_unsafe(position).insert(key());
        }
        let cx = Cx {
            traffic: Mutex::new(tx_traffic),
            ..Cx::default()
        };
        let sim = SettlementSimulation::new(cx);

        // When
        block_on(sim.update_position_traffic(&[change]));

        // Then
        let mut expected = traffic();
        for position in route_2().path.iter() {
            expected.mut_cell_unsafe(position).insert(key());
        }
        assert_eq!(*sim.cx.traffic.lock().unwrap(), expected);
    }

    #[test]
    fn updated_route_should_add_traffic_for_positions_not_in_old_route() {
        // Given
        let change = RouteChange::Updated {
            key: key(),
            old: route_2(),
            new: route_1(),
        };
        let mut tx_traffic = traffic();
        for position in route_2().path.iter() {
            tx_traffic.mut_cell_unsafe(position).insert(key());
        }
        let cx = Cx {
            traffic: Mutex::new(tx_traffic),
            ..Cx::default()
        };
        let sim = SettlementSimulation::new(cx);

        // When
        block_on(sim.update_position_traffic(&[change]));

        // Then
        let mut expected = traffic();
        for position in route_1().path.iter() {
            expected.mut_cell_unsafe(position).insert(key());
        }
        assert_eq!(*sim.cx.traffic.lock().unwrap(), expected);
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
        block_on(sim.update_position_traffic(&[change]));

        // Then
        assert_eq!(
            *sim.cx.refreshed_positions.lock().unwrap(),
            hashset! {v2(1, 3), v2(2, 3), v2(2, 4), v2(2, 5), v2(1, 5)}
        );
    }

    #[test]
    fn removed_route_should_remove_traffic_for_all_positions_in_route() {
        // Given
        let change = RouteChange::Removed {
            key: key(),
            route: route_1(),
        };
        let mut tx_traffic = traffic();
        for position in route_1().path.iter() {
            tx_traffic.mut_cell_unsafe(position).insert(key());
        }
        let sim = SettlementSimulation::new(Cx::default());

        // When
        block_on(sim.update_position_traffic(&[change]));

        // Then
        let expected = traffic();
        assert_eq!(*sim.cx.traffic.lock().unwrap(), expected);
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
        block_on(sim.update_position_traffic(&[change]));

        // Then
        assert_eq!(
            *sim.cx.refreshed_positions.lock().unwrap(),
            hashset! {v2(1, 3), v2(2, 3), v2(2, 4), v2(2, 5), v2(1, 5)}
        );
    }

    #[test]
    fn no_change_route_should_not_change_traffic() {
        // Given
        let change = RouteChange::NoChange {
            key: key(),
            route: route_1(),
        };
        let sim = SettlementSimulation::new(Cx::default());

        // When
        block_on(sim.update_position_traffic(&[change]));

        // Then
        assert_eq!(*sim.cx.traffic.lock().unwrap(), traffic());
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
        let mut tx_traffic = traffic();
        for position in route_1().path.iter() {
            tx_traffic.mut_cell_unsafe(position).insert(key_2);
        }
        let cx = Cx {
            traffic: Mutex::new(tx_traffic),
            ..Cx::default()
        };
        let sim = SettlementSimulation::new(cx);

        // When
        block_on(sim.update_position_traffic(&[change]));

        // Then
        let mut expected = traffic();
        for position in route_1().path.iter() {
            expected.mut_cell_unsafe(position).insert(key());
            expected.mut_cell_unsafe(position).insert(key_2);
        }
        assert_eq!(*sim.cx.traffic.lock().unwrap(), expected);
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
        let mut tx_traffic = traffic();
        for position in route_1().path.iter() {
            tx_traffic.mut_cell_unsafe(position).insert(key());
            tx_traffic.mut_cell_unsafe(position).insert(key_2);
        }
        let cx = Cx {
            traffic: Mutex::new(tx_traffic),
            ..Cx::default()
        };
        let sim = SettlementSimulation::new(cx);

        // When
        block_on(sim.update_position_traffic(&[change]));

        // Then
        let mut expected = traffic();
        for position in route_1().path.iter() {
            expected.mut_cell_unsafe(position).insert(key_2);
        }
        assert_eq!(*sim.cx.traffic.lock().unwrap(), expected);
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
        block_on(sim.update_position_traffic(&[change_1, change_2]));

        // Then
        assert_eq!(
            *sim.cx.refreshed_positions.lock().unwrap(),
            hashset! {v2(1, 3), v2(1, 4), v2(2, 3), v2(2, 4), v2(2, 5), v2(1, 5)}
        );
    }
}
