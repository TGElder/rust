use super::*;
use crate::game::traits::Micros;
use crate::pathfinder::traits::{ClosestTargetResult, ClosestTargets, InBounds};
use crate::route::{Route, RouteKey, RouteSet, RouteSetKey};
use crate::simulation::game_event_consumers::target_set;
use commons::get_corners;

const HANDLE: &str = "get_routes";

pub struct GetRoutes<G, P>
where
    G: Micros,
    P: ClosestTargets + InBounds + Send + Sync,
{
    game: UpdateSender<G>,
    pathfinder: Arc<RwLock<P>>,
}

#[async_trait]
impl<G, P> Processor for GetRoutes<G, P>
where
    G: Micros,
    P: ClosestTargets + InBounds + Send + Sync,
{
    async fn process(&mut self, mut state: State, instruction: &Instruction) -> State {
        let demand = match instruction {
            Instruction::GetRoutes(demand) => *demand,
            _ => return state,
        };
        let micros = self.game_micros().await;
        let route_set: RouteSet = routes(micros, &demand, self.closest_targets(&demand)).collect();
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

impl<G, P> GetRoutes<G, P>
where
    G: Micros,
    P: ClosestTargets + InBounds + Send + Sync,
{
    pub fn new(game: &UpdateSender<G>, pathfinder: &Arc<RwLock<P>>) -> GetRoutes<G, P> {
        GetRoutes {
            game: game.clone_with_handle(HANDLE),
            pathfinder: pathfinder.clone(),
        }
    }

    async fn game_micros(&mut self) -> u128 {
        self.game.update(|game| *game.micros()).await
    }

    fn closest_targets(&self, demand: &Demand) -> Vec<ClosestTargetResult> {
        if demand.sources == 0 || demand.quantity == 0 {
            return vec![];
        }
        let target_set = target_set(demand.resource);
        let sources = demand.sources;
        let pathfinder = self.pathfinder.read().unwrap();
        let corners: Vec<V2<usize>> = get_corners(&demand.position)
            .into_iter()
            .filter(|corner| pathfinder.in_bounds(corner))
            .collect();
        pathfinder.closest_targets(&corners, &target_set, sources)
    }
}

fn routes<'a>(
    start_micros: u128,
    demand: &'a Demand,
    closest_targets: Vec<ClosestTargetResult>,
) -> impl Iterator<Item = (RouteKey, Route)> + 'a {
    closest_targets
        .into_iter()
        .map(move |target| route(start_micros, demand, target))
}

fn route(start_micros: u128, demand: &Demand, target: ClosestTargetResult) -> (RouteKey, Route) {
    (
        RouteKey {
            settlement: demand.position,
            resource: demand.resource,
            destination: target.position,
        },
        Route {
            path: target.path,
            start_micros,
            duration: target.duration,
            traffic: demand.quantity,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::resource::Resource;
    use commons::futures::executor::block_on;
    use commons::update::UpdateProcess;
    use commons::{same_elements, v2};
    use std::collections::HashMap;
    use std::time::Duration;

    #[test]
    fn test() {
        struct MockPathfinder {}

        impl ClosestTargets for MockPathfinder {
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

        impl InBounds for MockPathfinder {
            fn in_bounds(&self, position: &V2<usize>) -> bool {
                *position != v2(2, 4)
            }
        }

        // Given
        let game = UpdateProcess::new(101);
        let pathfinder = Arc::new(RwLock::new(MockPathfinder {}));
        let mut processor = GetRoutes::new(&game.tx(), &pathfinder);
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
                duration: Duration::from_secs(2),
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
                duration: Duration::from_secs(4),
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
        game.shutdown();
    }

    #[test]
    fn test_no_closest_targets() {
        struct MockPathfinder {}

        impl ClosestTargets for MockPathfinder {
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

        impl InBounds for MockPathfinder {
            fn in_bounds(&self, position: &V2<usize>) -> bool {
                *position != v2(2, 4)
            }
        }

        // Given
        let game = UpdateProcess::new(101);
        let pathfinder = Arc::new(RwLock::new(MockPathfinder {}));
        let mut processor = GetRoutes::new(&game.tx(), &pathfinder);
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
        game.shutdown();
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
        let game = UpdateProcess::new(101);
        let pathfinder = Arc::new(RwLock::new(PanicPathfinder {}));
        let mut processor = GetRoutes::new(&game.tx(), &pathfinder);
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
        game.shutdown();
    }

    #[test]
    fn zero_quantity_route_should_return_empty_route_set_and_should_not_call_pathfinder() {
        // Given
        let game = UpdateProcess::new(101);
        let pathfinder = Arc::new(RwLock::new(PanicPathfinder {}));
        let mut processor = GetRoutes::new(&game.tx(), &pathfinder);
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
        game.shutdown();
    }
}
