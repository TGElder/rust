use crate::pathfinder::ClosestTargetResult;
use crate::route::{Route, RouteKey, RouteSet, RouteSetKey};
use crate::simulation::settlement::demand::Demand;
use crate::simulation::settlement::model::Routes;
use crate::simulation::settlement::SettlementSimulation;
use crate::traits::{ClosestTargetsForRoutes, CostOfPath, InBoundsForRoutes, Micros, WithBridges};
use crate::travel_duration::TravelDuration;
use commons::grid::get_corners;
use commons::V2;
use std::collections::HashMap;
use std::time::Duration;

impl<T, D> SettlementSimulation<T, D>
where
    T: ClosestTargetsForRoutes + CostOfPath + InBoundsForRoutes + Micros + WithBridges,
    D: TravelDuration,
{
    pub async fn get_routes(&self, demand: Demand) -> Routes {
        let micros = self.cx.micros().await;
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

        let target_set = demand.resource.name();
        let sources = demand.sources;
        let corners_in_bounds = self.corners_in_bound(&demand.position).await;
        self.cx
            .closest_targets(&corners_in_bounds, &target_set, sources)
            .await
    }

    async fn corners_in_bound(&self, position: &V2<usize>) -> Vec<V2<usize>> {
        let mut out = vec![];
        for corner in get_corners(&position) {
            if self.cx.in_bounds(&corner).await {
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
        let bridges = self.cx.with_bridges(|bridges| (*bridges).clone()).await;
        self.cx
            .cost_of_path(self.travel_duration.as_ref(), &bridges, path)
            .await
            .expect("Found route but not duration!")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::bridge::Bridges;
    use crate::resource::Resource;
    use crate::travel_duration::TravelDuration;
    use crate::world::World;
    use commons::async_trait::async_trait;
    use commons::{same_elements, v2};
    use futures::executor::block_on;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    #[derive(Default)]
    struct HappyPathTx {
        closest_targets: Vec<ClosestTargetResult>,
        bridges: Mutex<Bridges>,
    }

    #[async_trait]
    impl Micros for HappyPathTx {
        async fn micros(&self) -> u128 {
            101
        }
    }

    #[async_trait]
    impl CostOfPath for HappyPathTx {
        async fn cost_of_path<D>(&self, _: &D, _: &Bridges, _: &[V2<usize>]) -> Option<Duration>
        where
            D: TravelDuration,
        {
            Some(Duration::from_secs(303))
        }
    }

    #[async_trait]
    impl ClosestTargetsForRoutes for HappyPathTx {
        async fn closest_targets(
            &self,
            positions: &[V2<usize>],
            target_set: &str,
            _: usize,
        ) -> Vec<ClosestTargetResult> {
            assert!(same_elements(positions, &[v2(1, 3), v2(2, 3), v2(1, 4)]));
            assert_eq!(target_set, "coal");
            self.closest_targets.clone()
        }
    }

    #[async_trait]
    impl InBoundsForRoutes for HappyPathTx {
        async fn in_bounds(&self, position: &V2<usize>) -> bool {
            *position != v2(2, 4)
        }
    }

    #[async_trait]
    impl WithBridges for HappyPathTx {
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

    struct PanicTravelDuration {}

    impl TravelDuration for PanicTravelDuration {
        fn get_duration(&self, _: &World, _: &V2<usize>, _: &V2<usize>) -> Option<Duration> {
            panic!("Not expecting travel duration to be used!");
        }

        fn min_duration(&self) -> Duration {
            panic!("Not expecting travel duration to be used!");
        }

        fn max_duration(&self) -> Duration {
            panic!("Not expecting travel duration to be used!");
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
        let sim = SettlementSimulation::new(
            HappyPathTx {
                closest_targets,
                ..HappyPathTx::default()
            },
            Arc::new(PanicTravelDuration {}),
        );
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
        let sim = SettlementSimulation::new(
            HappyPathTx {
                closest_targets,
                ..HappyPathTx::default()
            },
            Arc::new(PanicTravelDuration {}),
        );
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
        let sim = SettlementSimulation::new(
            HappyPathTx {
                closest_targets,
                ..HappyPathTx::default()
            },
            Arc::new(PanicTravelDuration {}),
        );
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
    impl CostOfPath for PanicPathfinderTx {
        async fn cost_of_path<D>(&self, _: &D, _: &Bridges, _: &[V2<usize>]) -> Option<Duration>
        where
            D: TravelDuration,
        {
            panic!("cost_of_path was called!");
        }
    }

    #[async_trait]
    impl ClosestTargetsForRoutes for PanicPathfinderTx {
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
    impl InBoundsForRoutes for PanicPathfinderTx {
        async fn in_bounds(&self, _: &V2<usize>) -> bool {
            panic!("in_bounds was called!");
        }
    }

    #[async_trait]
    impl WithBridges for PanicPathfinderTx {
        async fn with_bridges<F, O>(&self, _: F) -> O
        where
            F: FnOnce(&Bridges) -> O + Send,
        {
            panic!("with_bridges was called!");
        }

        async fn mut_bridges<F, O>(&self, _: F) -> O
        where
            F: FnOnce(&mut Bridges) -> O + Send,
        {
            panic!("mut_bridges was called!");
        }
    }

    #[test]
    fn zero_source_route_should_return_empty_route_set_and_should_not_call_pathfinder() {
        // Given
        let sim = SettlementSimulation::new(PanicPathfinderTx {}, Arc::new(PanicTravelDuration {}));
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
        let sim = SettlementSimulation::new(PanicPathfinderTx {}, Arc::new(PanicTravelDuration {}));
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
