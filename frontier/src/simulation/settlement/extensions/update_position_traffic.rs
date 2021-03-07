use crate::route::{Route, RouteKey};
use crate::simulation::settlement::model::RouteChange;
use crate::simulation::settlement::SettlementSimulation;
use crate::traits::WithTraffic;
use commons::grid::Grid;
use commons::V2;
use futures::future::join_all;
use std::collections::HashSet;

impl<T> SettlementSimulation<T>
where
    T: WithTraffic,
{
    pub async fn update_all_position_traffic(&self, route_changes: &[RouteChange]) {
        join_all(
            route_changes
                .iter()
                .map(|route_change| self.update_position_traffic(route_change)),
        )
        .await;
    }

    async fn update_position_traffic(&self, route_change: &RouteChange) {
        match route_change {
            RouteChange::New { key, route } => self.new_position_traffic(&key, &route).await,
            RouteChange::Updated { key, old, new } => {
                self.updated_position_traffic(&key, &old, &new).await
            }
            RouteChange::Removed { key, route } => {
                self.removed_position_traffic(&key, &route).await
            }
            _ => (),
        }
    }

    async fn new_position_traffic(&self, key: &RouteKey, route: &Route) {
        self.cx
            .mut_traffic(|traffic| {
                for position in route.path.iter() {
                    traffic.mut_cell_unsafe(&position).insert(*key);
                }
            })
            .await;
    }

    async fn updated_position_traffic(&self, key: &RouteKey, old: &Route, new: &Route) {
        let old: HashSet<&V2<usize>> = old.path.iter().collect();
        let new: HashSet<&V2<usize>> = new.path.iter().collect();

        let added = new.difference(&old).cloned();
        let removed = old.difference(&new).cloned();

        self.cx
            .mut_traffic(|traffic| {
                for position in added {
                    traffic.mut_cell_unsafe(&position).insert(*key);
                }

                for position in removed {
                    traffic.mut_cell_unsafe(&position).remove(key);
                }
            })
            .await;
    }

    async fn removed_position_traffic(&self, key: &RouteKey, route: &Route) {
        self.cx
            .mut_traffic(|traffic| {
                for position in route.path.iter() {
                    traffic.mut_cell_unsafe(&position).remove(key);
                }
            })
            .await;
    }
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
        traffic: Mutex<Traffic>,
    }

    impl Default for Cx {
        fn default() -> Self {
            Cx {
                traffic: Mutex::new(traffic()),
            }
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
    fn new_route_should_add_traffic_for_all_positions_in_route() {
        // Given
        let change = RouteChange::New {
            key: key(),
            route: route_1(),
        };
        let sim = SettlementSimulation::new(Cx::default());

        // When
        block_on(sim.update_all_position_traffic(&[change]));

        // Then
        let mut expected = traffic();
        for position in route_1().path.iter() {
            expected.mut_cell_unsafe(position).insert(key());
        }
        assert_eq!(*sim.cx.traffic.lock().unwrap(), expected);
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
        };
        let sim = SettlementSimulation::new(cx);

        // When
        block_on(sim.update_all_position_traffic(&[change]));

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
        };
        let sim = SettlementSimulation::new(cx);

        // When
        block_on(sim.update_all_position_traffic(&[change]));

        // Then
        let mut expected = traffic();
        for position in route_1().path.iter() {
            expected.mut_cell_unsafe(position).insert(key());
        }
        assert_eq!(*sim.cx.traffic.lock().unwrap(), expected);
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
        block_on(sim.update_all_position_traffic(&[change]));

        // Then
        let expected = traffic();
        assert_eq!(*sim.cx.traffic.lock().unwrap(), expected);
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
        block_on(sim.update_all_position_traffic(&[change]));

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
        };
        let sim = SettlementSimulation::new(cx);

        // When
        block_on(sim.update_all_position_traffic(&[change]));

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
        };
        let sim = SettlementSimulation::new(cx);

        // When
        block_on(sim.update_all_position_traffic(&[change]));

        // Then
        let mut expected = traffic();
        for position in route_1().path.iter() {
            expected.mut_cell_unsafe(position).insert(key_2);
        }
        assert_eq!(*sim.cx.traffic.lock().unwrap(), expected);
    }
}
