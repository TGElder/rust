use super::*;
use crate::game::traits::Micros;
use crate::pathfinder::traits::{ClosestTargetResult, ClosestTargets, InBounds, LowestDuration};
use crate::route::{Route, RouteKey, RouteSet, RouteSetKey};
use crate::simulation::game_event_consumers::target_set;
use commons::grid::get_corners;
use std::time::Duration;

const NAME: &str = "get_routes";

pub struct GetRoutes<G, P, Q>
where
    G: Micros + Send,
    P: ClosestTargets + InBounds + Send + Sync,
    Q: LowestDuration + Send + Sync,
{
    game: FnSender<G>,
    route_pathfinder: Arc<RwLock<P>>,
    duration_pathfinder: Arc<RwLock<Q>>,
}

#[async_trait]
impl<G, P, Q> Processor for GetRoutes<G, P, Q>
where
    G: Micros + Send,
    P: ClosestTargets + InBounds + Send + Sync,
    Q: LowestDuration + Send + Sync,
{
    async fn process(&mut self, mut state: State, instruction: &Instruction) -> State {
        let demand = match instruction {
            Instruction::GetRoutes(demand) => *demand,
            _ => return state,
        };
        let micros = self.game_micros().await;
        let route_set: RouteSet = self
            .routes(micros, &demand, self.closest_targets(&demand))
            .collect();
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

impl<G, P, Q> GetRoutes<G, P, Q>
where
    G: Micros + Send,
    P: ClosestTargets + InBounds + Send + Sync,
    Q: LowestDuration + Send + Sync,
{
    pub fn new(
        game: &FnSender<G>,
        route_pathfinder: &Arc<RwLock<P>>,
        duration_pathfinder: &Arc<RwLock<Q>>,
    ) -> GetRoutes<G, P, Q> {
        GetRoutes {
            game: game.clone_with_name(NAME),
            route_pathfinder: route_pathfinder.clone(),
            duration_pathfinder: duration_pathfinder.clone(),
        }
    }

    async fn game_micros(&mut self) -> u128 {
        self.game.send(|game| *game.micros()).await
    }

    fn closest_targets(&self, demand: &Demand) -> Vec<ClosestTargetResult> {
        if demand.sources == 0 || demand.quantity == 0 {
            return vec![];
        }
        let target_set = target_set(demand.resource);
        let sources = demand.sources;
        let pathfinder = self.route_pathfinder.read().unwrap();
        let corners: Vec<V2<usize>> = get_corners(&demand.position)
            .into_iter()
            .filter(|corner| pathfinder.in_bounds(corner))
            .collect();
        pathfinder.closest_targets(&corners, &target_set, sources)
    }

    fn routes<'a>(
        &'a self,
        start_micros: u128,
        demand: &'a Demand,
        closest_targets: Vec<ClosestTargetResult>,
    ) -> impl Iterator<Item = (RouteKey, Route)> + 'a {
        closest_targets
            .into_iter()
            .take(demand.sources)
            .map(move |target| self.route(start_micros, demand, target))
    }

    fn route(
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
                duration: self.route_duration(&target.path),
                path: target.path,
                start_micros,
                traffic: demand.quantity,
            },
        )
    }

    fn route_duration(&self, path: &[V2<usize>]) -> Duration {
        self.duration_pathfinder
            .read()
            .unwrap()
            .lowest_duration(&path)
            .expect("Route pathfinder found route but duration pathfinder did not!")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::resource::Resource;
    use commons::fn_sender::FnThread;
    use commons::{same_elements, v2};
    use futures::executor::block_on;
    use std::collections::HashMap;
    use std::time::Duration;

    struct MockDurationPathfinder {}

    impl LowestDuration for MockDurationPathfinder {
        fn lowest_duration(&self, _: &[V2<usize>]) -> Option<Duration> {
            Some(Duration::from_secs(303))
        }
    }

    #[test]
    fn test() {
        struct MockRoutePathfinder {}

        impl ClosestTargets for MockRoutePathfinder {
            fn init_targets(&mut self, _: String) {}

            fn load_target(&mut self, _: &str, _: &V2<usize>, _: bool) {}

            fn closest_targets(
                &self,
                positions: &[V2<usize>],
                target_set: &str,
                n_closest: usize,
            ) -> Vec<ClosestTargetResult> {
                assert!(same_elements(positions, &[v2(1, 3), v2(2, 3), v2(1, 4)]));
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

        impl InBounds for MockRoutePathfinder {
            fn in_bounds(&self, position: &V2<usize>) -> bool {
                *position != v2(2, 4)
            }
        }

        // Given
        let game = FnThread::new(101);
        let route_pathfinder = Arc::new(RwLock::new(MockRoutePathfinder {}));
        let duration_pathfinder = Arc::new(RwLock::new(MockDurationPathfinder {}));
        let mut processor = GetRoutes::new(&game.tx(), &route_pathfinder, &duration_pathfinder);
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

        // Finally
        game.join();
    }

    #[test]
    fn test_no_closest_targets() {
        struct MockRoutePathfinder {}

        impl ClosestTargets for MockRoutePathfinder {
            fn init_targets(&mut self, _: String) {}

            fn load_target(&mut self, _: &str, _: &V2<usize>, _: bool) {}

            fn closest_targets(
                &self,
                positions: &[V2<usize>],
                target_set: &str,
                n_closest: usize,
            ) -> Vec<ClosestTargetResult> {
                assert!(same_elements(positions, &[v2(1, 3), v2(2, 3), v2(1, 4)]));
                assert_eq!(target_set, "resource-coal");
                assert_eq!(n_closest, 2);
                vec![]
            }
        }

        impl InBounds for MockRoutePathfinder {
            fn in_bounds(&self, position: &V2<usize>) -> bool {
                *position != v2(2, 4)
            }
        }

        // Given
        let game = FnThread::new(101);
        let route_pathfinder = Arc::new(RwLock::new(MockRoutePathfinder {}));
        let duration_pathfinder = Arc::new(RwLock::new(MockDurationPathfinder {}));
        let mut processor = GetRoutes::new(&game.tx(), &route_pathfinder, &duration_pathfinder);
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

        // Finally
        game.join();
    }

    struct PanicPathfinder {}

    impl ClosestTargets for PanicPathfinder {
        fn init_targets(&mut self, _: String) {}

        fn load_target(&mut self, _: &str, _: &V2<usize>, _: bool) {}

        fn closest_targets(&self, _: &[V2<usize>], _: &str, _: usize) -> Vec<ClosestTargetResult> {
            panic!("closest_targets was called!");
        }
    }

    impl InBounds for PanicPathfinder {
        fn in_bounds(&self, _: &V2<usize>) -> bool {
            panic!("in_bounds was called!");
        }
    }

    #[test]
    fn zero_source_route_should_return_empty_route_set_and_should_not_call_pathfinder() {
        // Given
        let game = FnThread::new(101);
        let route_pathfinder = Arc::new(RwLock::new(PanicPathfinder {}));
        let duration_pathfinder = Arc::new(RwLock::new(MockDurationPathfinder {}));
        let mut processor = GetRoutes::new(&game.tx(), &route_pathfinder, &duration_pathfinder);
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

        // Finally
        game.join();
    }

    #[test]
    fn zero_quantity_route_should_return_empty_route_set_and_should_not_call_pathfinder() {
        // Given
        let game = FnThread::new(101);
        let route_pathfinder = Arc::new(RwLock::new(PanicPathfinder {}));
        let duration_pathfinder = Arc::new(RwLock::new(MockDurationPathfinder {}));
        let mut processor = GetRoutes::new(&game.tx(), &route_pathfinder, &duration_pathfinder);
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

        // Finally
        game.join();
    }

    #[test]
    fn test_more_closest_targets_than_requested() {
        struct MockRoutePathfinder {}

        impl ClosestTargets for MockRoutePathfinder {
            fn init_targets(&mut self, _: String) {}

            fn load_target(&mut self, _: &str, _: &V2<usize>, _: bool) {}

            fn closest_targets(
                &self,
                positions: &[V2<usize>],
                target_set: &str,
                n_closest: usize,
            ) -> Vec<ClosestTargetResult> {
                assert!(same_elements(positions, &[v2(1, 3), v2(2, 3), v2(1, 4)]));
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

        impl InBounds for MockRoutePathfinder {
            fn in_bounds(&self, position: &V2<usize>) -> bool {
                *position != v2(2, 4)
            }
        }

        // Given
        let game = FnThread::new(101);
        let route_pathfinder = Arc::new(RwLock::new(MockRoutePathfinder {}));
        let duration_pathfinder = Arc::new(RwLock::new(MockDurationPathfinder {}));
        let mut processor = GetRoutes::new(&game.tx(), &route_pathfinder, &duration_pathfinder);
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

        // Finally
        game.join();
    }
}
