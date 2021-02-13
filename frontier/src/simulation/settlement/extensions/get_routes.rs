use crate::actors::target_set;
use crate::pathfinder::ClosestTargetResult;
use crate::route::{Route, RouteKey, RouteSet, RouteSetKey};
use crate::simulation::settlement::demand::Demand;
use crate::simulation::settlement::instruction::Routes;
use crate::simulation::settlement::SettlementSimulation;
use crate::traits::{
    ClosestTargetsWithPlannedRoads, InBoundsWithPlannedRoads, LowestDurationWithoutPlannedRoads,
    Micros,
};
use commons::grid::get_corners;
use commons::V2;
use std::collections::HashMap;
use std::time::Duration;

impl<T> SettlementSimulation<T>
where
    T: ClosestTargetsWithPlannedRoads
        + InBoundsWithPlannedRoads
        + LowestDurationWithoutPlannedRoads
        + Micros,
{
    pub async fn get_routes(&self, demand: Demand) -> Routes {
        let micros = self.tx.micros().await;
        let closest_targets = self.closest_targets(&demand).await;
        let route_set = self.route_set(micros, &demand, closest_targets).await;
        Routes {
            key: RouteSetKey {
                settlement: demand.position,
                resource: demand.resource,
            },
            route_set,
        }
    }

    async fn closest_targets(&self, demand: &Demand) -> Vec<ClosestTargetResult> {
        if demand.sources == 0 || demand.quantity == 0 {
            return vec![];
        }

        let target_set = target_set(demand.resource);
        let sources = demand.sources;
        let corners_in_bounds = self.corners_in_bound(&demand.position).await;
        self.tx
            .closest_targets(&corners_in_bounds, &target_set, sources)
            .await
    }

    async fn corners_in_bound(&self, position: &V2<usize>) -> Vec<V2<usize>> {
        let mut out = vec![];
        for corner in get_corners(&position) {
            if self.tx.in_bounds(&corner).await {
                out.push(corner);
            }
        }
        out
    }

    async fn route_set(
        &self,
        start_micros: u128,
        demand: &Demand,
        closest_targets: Vec<ClosestTargetResult>,
    ) -> RouteSet {
        let mut out = HashMap::new();
        for target in closest_targets {
            let (key, route) = self.route(start_micros, demand, target).await;
            out.insert(key, route);
            if out.len() == demand.sources {
                return out;
            }
        }
        out
    }

    async fn route(
        &self,
        start_micros: u128,
        demand: &Demand,
        target: ClosestTargetResult,
    ) -> (RouteKey, Route) {
        (
            RouteKey {
                settlement: demand.position,
                resource: demand.resource,
                destination: target.position,
            },
            Route {
                duration: self.route_duration(&target.path).await,
                path: target.path,
                start_micros,
                traffic: demand.quantity,
            },
        )
    }

    async fn route_duration(&self, path: &[V2<usize>]) -> Duration {
        self.tx
            .lowest_duration(path)
            .await
            .expect("Found route with planned roads but not without planned roads!")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::resource::Resource;
    use commons::async_trait::async_trait;
    use commons::{same_elements, v2};
    use futures::executor::block_on;
    use std::collections::HashMap;
    use std::time::Duration;

    struct HappyPathTx {
        closest_targets: Vec<ClosestTargetResult>,
    }

    #[async_trait]
    impl Micros for HappyPathTx {
        async fn micros(&self) -> u128 {
            101
        }
    }

    #[async_trait]
    impl LowestDurationWithoutPlannedRoads for HappyPathTx {
        async fn lowest_duration(&self, _: &[V2<usize>]) -> Option<Duration> {
            Some(Duration::from_secs(303))
        }
    }

    #[async_trait]
    impl ClosestTargetsWithPlannedRoads for HappyPathTx {
        async fn closest_targets(
            &self,
            positions: &[V2<usize>],
            target_set: &str,
            _: usize,
        ) -> Vec<ClosestTargetResult> {
            assert!(same_elements(positions, &[v2(1, 3), v2(2, 3), v2(1, 4)]));
            assert_eq!(target_set, "resource-coal");
            self.closest_targets.clone()
        }
    }

    #[async_trait]
    impl InBoundsWithPlannedRoads for HappyPathTx {
        async fn in_bounds(&self, position: &V2<usize>) -> bool {
            *position != v2(2, 4)
        }
    }

    #[test]
    fn test() {
        // Given
        let closest_targets = vec![
            ClosestTargetResult {
                position: v2(1, 5),
                path: vec![v2(1, 3), v2(1, 4), v2(1, 5)],
                duration: Duration::from_secs(2),
            },
            ClosestTargetResult {
                position: v2(5, 3),
                path: vec![v2(1, 3), v2(2, 3), v2(3, 3), v2(4, 3), v2(5, 3)],
                duration: Duration::from_secs(4),
            },
        ];
        let sim = SettlementSimulation::new(HappyPathTx { closest_targets });
        let demand = Demand {
            position: v2(1, 3),
            resource: Resource::Coal,
            sources: 2,
            quantity: 3,
        };

        // When
        let routes = block_on(sim.get_routes(demand));

        // Then
        let mut route_set = HashMap::new();
        route_set.insert(
            RouteKey {
                settlement: v2(1, 3),
                resource: Resource::Coal,
                destination: v2(1, 5),
            },
            Route {
                path: vec![v2(1, 3), v2(1, 4), v2(1, 5)],
                start_micros: 101,
                duration: Duration::from_secs(303),
                traffic: 3,
            },
        );
        route_set.insert(
            RouteKey {
                settlement: v2(1, 3),
                resource: Resource::Coal,
                destination: v2(5, 3),
            },
            Route {
                path: vec![v2(1, 3), v2(2, 3), v2(3, 3), v2(4, 3), v2(5, 3)],
                start_micros: 101,
                duration: Duration::from_secs(303),
                traffic: 3,
            },
        );

        assert_eq!(
            routes,
            Routes {
                key: RouteSetKey {
                    settlement: v2(1, 3),
                    resource: Resource::Coal
                },
                route_set
            }
        );
    }

    #[test]
    fn test_no_closest_targets() {
        // Given
        let closest_targets = vec![];
        let sim = SettlementSimulation::new(HappyPathTx { closest_targets });
        let demand = Demand {
            position: v2(1, 3),
            resource: Resource::Coal,
            sources: 2,
            quantity: 3,
        };

        // When
        let routes = block_on(sim.get_routes(demand));

        // Then
        assert_eq!(
            routes,
            Routes {
                key: RouteSetKey {
                    settlement: v2(1, 3),
                    resource: Resource::Coal
                },
                route_set: hashmap! {}
            }
        );
    }

    #[test]
    fn test_more_closest_targets_than_requested() {
        // Given
        let closest_targets = vec![
            ClosestTargetResult {
                position: v2(1, 5),
                path: vec![v2(1, 3), v2(1, 4), v2(1, 5)],
                duration: Duration::from_secs(2),
            },
            ClosestTargetResult {
                position: v2(5, 3),
                path: vec![v2(1, 3), v2(2, 3), v2(3, 3), v2(4, 3), v2(5, 3)],
                duration: Duration::from_secs(4),
            },
        ];
        let sim = SettlementSimulation::new(HappyPathTx { closest_targets });
        let demand = Demand {
            position: v2(1, 3),
            resource: Resource::Coal,
            sources: 1,
            quantity: 3,
        };

        // When
        let routes = block_on(sim.get_routes(demand));

        // Then
        let mut route_set = HashMap::new();
        route_set.insert(
            RouteKey {
                settlement: v2(1, 3),
                resource: Resource::Coal,
                destination: v2(1, 5),
            },
            Route {
                path: vec![v2(1, 3), v2(1, 4), v2(1, 5)],
                start_micros: 101,
                duration: Duration::from_secs(303),
                traffic: 3,
            },
        );

        assert_eq!(
            routes,
            Routes {
                key: RouteSetKey {
                    settlement: v2(1, 3),
                    resource: Resource::Coal
                },
                route_set
            }
        );
    }

    struct PanicPathfinderTx {}

    #[async_trait]
    impl Micros for PanicPathfinderTx {
        async fn micros(&self) -> u128 {
            101
        }
    }

    #[async_trait]
    impl LowestDurationWithoutPlannedRoads for PanicPathfinderTx {
        async fn lowest_duration(&self, _: &[V2<usize>]) -> Option<Duration> {
            Some(Duration::from_secs(303))
        }
    }

    #[async_trait]
    impl ClosestTargetsWithPlannedRoads for PanicPathfinderTx {
        async fn closest_targets(
            &self,
            _: &[V2<usize>],
            _: &str,
            _: usize,
        ) -> Vec<ClosestTargetResult> {
            panic!("closest_targets was called!");
        }
    }

    #[async_trait]
    impl InBoundsWithPlannedRoads for PanicPathfinderTx {
        async fn in_bounds(&self, _: &V2<usize>) -> bool {
            panic!("in_bounds was called!");
        }
    }

    #[test]
    fn zero_source_route_should_return_empty_route_set_and_should_not_call_pathfinder() {
        // Given
        let sim = SettlementSimulation::new(PanicPathfinderTx {});
        let demand = Demand {
            position: v2(1, 3),
            resource: Resource::Coal,
            sources: 0,
            quantity: 1,
        };

        // When
        let routes = block_on(sim.get_routes(demand));

        // Then
        assert_eq!(
            routes,
            Routes {
                key: RouteSetKey {
                    settlement: v2(1, 3),
                    resource: Resource::Coal
                },
                route_set: hashmap! {}
            }
        );
    }

    #[test]
    fn zero_quantity_route_should_return_empty_route_set_and_should_not_call_pathfinder() {
        // Given
        let sim = SettlementSimulation::new(PanicPathfinderTx {});
        let demand = Demand {
            position: v2(1, 3),
            resource: Resource::Coal,
            sources: 1,
            quantity: 0,
        };

        // When
        let routes = block_on(sim.get_routes(demand));

        // Then
        assert_eq!(
            routes,
            Routes {
                key: RouteSetKey {
                    settlement: v2(1, 3),
                    resource: Resource::Coal
                },
                route_set: hashmap! {}
            }
        );
    }
}
