use super::*;
use crate::actors::target_set;
use crate::pathfinder::ClosestTargetResult;
use crate::route::{Route, RouteKey, RouteSet, RouteSetKey};
use crate::traits::{
    ClosestTargetsWithPlannedRoads, InBoundsWithPlannedRoads, LowestDurationWithoutPlannedRoads,
    Micros,
};
use commons::grid::get_corners;
use std::collections::HashMap;
use std::time::Duration;

pub struct GetRoutes<T> {
    tx: T,
}

#[async_trait]
impl<T> Processor for GetRoutes<T>
where
    T: ClosestTargetsWithPlannedRoads
        + InBoundsWithPlannedRoads
        + LowestDurationWithoutPlannedRoads
        + Micros
        + Send
        + Sync,
{
    async fn process(&mut self, mut state: State, instruction: &Instruction) -> State {
        let demand = match instruction {
            Instruction::GetRoutes(demand) => *demand,
            _ => return state,
        };
        let micros = self.tx.micros().await;
        let closest_targets = self.closest_targets(&demand).await;
        let route_set = self.route_set(micros, &demand, closest_targets).await;
        state.instructions.push(Instruction::GetRouteChanges {
            key: RouteSetKey {
                settlement: demand.position,
                resource: demand.resource,
            },
            route_set,
        });
        state
    }
}

impl<T> GetRoutes<T>
where
    T: ClosestTargetsWithPlannedRoads
        + InBoundsWithPlannedRoads
        + LowestDurationWithoutPlannedRoads
        + Micros
        + Send
        + Sync,
{
    pub fn new(tx: T) -> GetRoutes<T> {
        GetRoutes { tx }
    }

    async fn closest_targets(&self, demand: &Demand) -> Vec<ClosestTargetResult> {
        if demand.sources == 0 || demand.quantity == 0 {
            return vec![];
        }
        let target_set = target_set(demand.resource);
        let sources = demand.sources;

        let mut corners_in_bounds = vec![];
        for corner in get_corners(&demand.position) {
            if self.tx.in_bounds(corner).await {
                corners_in_bounds.push(corner);
            }
        }

        self.tx
            .closest_targets(corners_in_bounds, target_set, sources)
            .await
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
                duration: self.route_duration(target.path.clone()).await,
                path: target.path,
                start_micros,
                traffic: demand.quantity,
            },
        )
    }

    async fn route_duration(&self, path: Vec<V2<usize>>) -> Duration {
        self.tx
            .lowest_duration(path)
            .await
            .expect("Route pathfinder found route but duration pathfinder did not!")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::resource::Resource;
    use commons::{same_elements, v2};
    use futures::executor::block_on;
    use std::collections::HashMap;
    use std::time::Duration;

    #[test]
    fn test() {
        struct Tx {}

        #[async_trait]
        impl Micros for Tx {
            async fn micros(&self) -> u128 {
                101
            }
        }

        #[async_trait]
        impl LowestDurationWithoutPlannedRoads for Tx {
            async fn lowest_duration(&self, _: Vec<V2<usize>>) -> Option<Duration> {
                Some(Duration::from_secs(303))
            }
        }

        #[async_trait]
        impl ClosestTargetsWithPlannedRoads for Tx {
            async fn closest_targets(
                &self,
                positions: Vec<V2<usize>>,
                target_set: String,
                n_closest: usize,
            ) -> Vec<ClosestTargetResult> {
                assert!(same_elements(&positions, &[v2(1, 3), v2(2, 3), v2(1, 4)]));
                assert_eq!(target_set, "resource-coal");
                assert_eq!(n_closest, 2);
                vec![
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
                ]
            }
        }

        #[async_trait]
        impl InBoundsWithPlannedRoads for Tx {
            async fn in_bounds(&self, position: V2<usize>) -> bool {
                position != v2(2, 4)
            }
        }

        // Given
        let mut processor = GetRoutes::new(Tx {});
        let demand = Demand {
            position: v2(1, 3),
            resource: Resource::Coal,
            sources: 2,
            quantity: 3,
        };

        // When
        let state = block_on(processor.process(State::default(), &Instruction::GetRoutes(demand)));

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
            state.instructions,
            vec![Instruction::GetRouteChanges {
                key: RouteSetKey {
                    settlement: v2(1, 3),
                    resource: Resource::Coal
                },
                route_set
            }]
        );
    }

    #[test]
    fn test_no_closest_targets() {
        struct Tx {}

        #[async_trait]
        impl Micros for Tx {
            async fn micros(&self) -> u128 {
                101
            }
        }

        #[async_trait]
        impl LowestDurationWithoutPlannedRoads for Tx {
            async fn lowest_duration(&self, _: Vec<V2<usize>>) -> Option<Duration> {
                Some(Duration::from_secs(303))
            }
        }

        #[async_trait]
        impl ClosestTargetsWithPlannedRoads for Tx {
            async fn closest_targets(
                &self,
                positions: Vec<V2<usize>>,
                target_set: String,
                n_closest: usize,
            ) -> Vec<ClosestTargetResult> {
                assert!(same_elements(&positions, &[v2(1, 3), v2(2, 3), v2(1, 4)]));
                assert_eq!(target_set, "resource-coal");
                assert_eq!(n_closest, 2);
                vec![]
            }
        }

        #[async_trait]
        impl InBoundsWithPlannedRoads for Tx {
            async fn in_bounds(&self, position: V2<usize>) -> bool {
                position != v2(2, 4)
            }
        }

        // Given
        let mut processor = GetRoutes::new(Tx {});
        let demand = Demand {
            position: v2(1, 3),
            resource: Resource::Coal,
            sources: 2,
            quantity: 3,
        };

        // When
        let state = block_on(processor.process(State::default(), &Instruction::GetRoutes(demand)));

        // Then
        assert_eq!(
            state.instructions,
            vec![Instruction::GetRouteChanges {
                key: RouteSetKey {
                    settlement: v2(1, 3),
                    resource: Resource::Coal
                },
                route_set: hashmap! {}
            }]
        );
    }

    #[test]
    fn zero_source_route_should_return_empty_route_set_and_should_not_call_pathfinder() {
        struct Tx {}

        #[async_trait]
        impl Micros for Tx {
            async fn micros(&self) -> u128 {
                101
            }
        }

        #[async_trait]
        impl LowestDurationWithoutPlannedRoads for Tx {
            async fn lowest_duration(&self, _: Vec<V2<usize>>) -> Option<Duration> {
                Some(Duration::from_secs(303))
            }
        }

        #[async_trait]
        impl ClosestTargetsWithPlannedRoads for Tx {
            async fn closest_targets(
                &self,
                _: Vec<V2<usize>>,
                _: String,
                _: usize,
            ) -> Vec<ClosestTargetResult> {
                panic!("closest_targets was called!");
            }
        }

        #[async_trait]
        impl InBoundsWithPlannedRoads for Tx {
            async fn in_bounds(&self, _: V2<usize>) -> bool {
                panic!("in_bounds was called!");
            }
        }

        // Given
        let mut processor = GetRoutes::new(Tx {});
        let demand = Demand {
            position: v2(1, 3),
            resource: Resource::Coal,
            sources: 0,
            quantity: 1,
        };

        // When
        let state = block_on(processor.process(State::default(), &Instruction::GetRoutes(demand)));

        // Then
        assert_eq!(
            state.instructions,
            vec![Instruction::GetRouteChanges {
                key: RouteSetKey {
                    settlement: v2(1, 3),
                    resource: Resource::Coal
                },
                route_set: hashmap! {}
            }]
        );
    }

    #[test]
    fn zero_quantity_route_should_return_empty_route_set_and_should_not_call_pathfinder() {
        struct Tx {}

        #[async_trait]
        impl Micros for Tx {
            async fn micros(&self) -> u128 {
                101
            }
        }

        #[async_trait]
        impl LowestDurationWithoutPlannedRoads for Tx {
            async fn lowest_duration(&self, _: Vec<V2<usize>>) -> Option<Duration> {
                Some(Duration::from_secs(303))
            }
        }

        #[async_trait]
        impl ClosestTargetsWithPlannedRoads for Tx {
            async fn closest_targets(
                &self,
                _: Vec<V2<usize>>,
                _: String,
                _: usize,
            ) -> Vec<ClosestTargetResult> {
                panic!("closest_targets was called!");
            }
        }

        #[async_trait]
        impl InBoundsWithPlannedRoads for Tx {
            async fn in_bounds(&self, _: V2<usize>) -> bool {
                panic!("in_bounds was called!");
            }
        }

        // Given
        let mut processor = GetRoutes::new(Tx {});
        let demand = Demand {
            position: v2(1, 3),
            resource: Resource::Coal,
            sources: 1,
            quantity: 0,
        };

        // When
        let state = block_on(processor.process(State::default(), &Instruction::GetRoutes(demand)));

        // Then
        assert_eq!(
            state.instructions,
            vec![Instruction::GetRouteChanges {
                key: RouteSetKey {
                    settlement: v2(1, 3),
                    resource: Resource::Coal
                },
                route_set: hashmap! {}
            }]
        );
    }

    #[test]
    fn test_more_closest_targets_than_requested() {
        struct Tx {}

        #[async_trait]
        impl Micros for Tx {
            async fn micros(&self) -> u128 {
                101
            }
        }

        #[async_trait]
        impl LowestDurationWithoutPlannedRoads for Tx {
            async fn lowest_duration(&self, _: Vec<V2<usize>>) -> Option<Duration> {
                Some(Duration::from_secs(303))
            }
        }

        #[async_trait]
        impl ClosestTargetsWithPlannedRoads for Tx {
            async fn closest_targets(
                &self,
                positions: Vec<V2<usize>>,
                target_set: String,
                n_closest: usize,
            ) -> Vec<ClosestTargetResult> {
                assert!(same_elements(&positions, &[v2(1, 3), v2(2, 3), v2(1, 4)]));
                assert_eq!(target_set, "resource-coal");
                assert_eq!(n_closest, 1);
                vec![
                    ClosestTargetResult {
                        position: v2(1, 5),
                        path: vec![v2(1, 3), v2(1, 4), v2(1, 5)],
                        duration: Duration::from_secs(2),
                    },
                    ClosestTargetResult {
                        position: v2(2, 4),
                        path: vec![v2(1, 3), v2(1, 4), v2(2, 4)],
                        duration: Duration::from_secs(2),
                    },
                ]
            }
        }

        #[async_trait]
        impl InBoundsWithPlannedRoads for Tx {
            async fn in_bounds(&self, position: V2<usize>) -> bool {
                position != v2(2, 4)
            }
        }

        // Given
        let mut processor = GetRoutes::new(Tx {});
        let demand = Demand {
            position: v2(1, 3),
            resource: Resource::Coal,
            sources: 1,
            quantity: 3,
        };

        // When
        let state = block_on(processor.process(State::default(), &Instruction::GetRoutes(demand)));

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
            state.instructions,
            vec![Instruction::GetRouteChanges {
                key: RouteSetKey {
                    settlement: v2(1, 3),
                    resource: Resource::Coal
                },
                route_set
            }]
        );
    }
}
