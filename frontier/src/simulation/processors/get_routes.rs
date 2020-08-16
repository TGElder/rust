use super::*;
use crate::pathfinder::traits::{ClosestTargetResult, ClosestTargets, InBounds};
use crate::route::{Route, RouteKey, RouteSet, RouteSetKey};
use crate::simulation::game_event_consumers::target_set;
use commons::get_corners;

pub struct GetRoutes<P>
where
    P: ClosestTargets + InBounds,
{
    pathfinder: Arc<RwLock<P>>,
}

impl<P> Processor for GetRoutes<P>
where
    P: ClosestTargets + InBounds,
{
    fn process(&mut self, mut state: State, instruction: &Instruction) -> State {
        let demand = match instruction {
            Instruction::GetRoutes(demand) => *demand,
            _ => return state,
        };
        let route_set: RouteSet = routes(&demand, self.closest_targets(&demand)).collect();
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

impl<P> GetRoutes<P>
where
    P: ClosestTargets + InBounds,
{
    pub fn new(pathfinder: &Arc<RwLock<P>>) -> GetRoutes<P> {
        GetRoutes {
            pathfinder: pathfinder.clone(),
        }
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
    demand: &'a Demand,
    closest_targets: Vec<ClosestTargetResult>,
) -> impl Iterator<Item = (RouteKey, Route)> + 'a {
    closest_targets
        .into_iter()
        .map(move |target| route(demand, target))
}

fn route(demand: &Demand, target: ClosestTargetResult) -> (RouteKey, Route) {
    (
        RouteKey {
            settlement: demand.position,
            resource: demand.resource,
            destination: target.position,
        },
        Route {
            path: target.path,
            start_micros: 0,
            duration: target.duration,
            traffic: demand.quantity,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::resource::Resource;
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

        let pathfinder = Arc::new(RwLock::new(MockPathfinder {}));
        let mut processor = GetRoutes::new(&pathfinder);
        let demand = Demand {
            position: v2(1, 3),
            resource: Resource::Coal,
            sources: 2,
            quantity: 3,
        };

        let state = processor.process(State::default(), &Instruction::GetRoutes(demand));

        let mut route_set = HashMap::new();
        route_set.insert(
            RouteKey {
                settlement: v2(1, 3),
                resource: Resource::Coal,
                destination: v2(1, 5),
            },
            Route {
                path: vec![v2(1, 3), v2(1, 4), v2(1, 5)],
                start_micros: 0,
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
                start_micros: 0,
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

        let pathfinder = Arc::new(RwLock::new(MockPathfinder {}));
        let mut processor = GetRoutes::new(&pathfinder);
        let demand = Demand {
            position: v2(1, 3),
            resource: Resource::Coal,
            sources: 2,
            quantity: 3,
        };

        let state = processor.process(State::default(), &Instruction::GetRoutes(demand));

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
        let pathfinder = Arc::new(RwLock::new(PanicPathfinder {}));
        let mut processor = GetRoutes::new(&pathfinder);
        let demand = Demand {
            position: v2(1, 3),
            resource: Resource::Coal,
            sources: 0,
            quantity: 1,
        };

        let state = processor.process(State::default(), &Instruction::GetRoutes(demand));

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
        let pathfinder = Arc::new(RwLock::new(PanicPathfinder {}));
        let mut processor = GetRoutes::new(&pathfinder);
        let demand = Demand {
            position: v2(1, 3),
            resource: Resource::Coal,
            sources: 1,
            quantity: 0,
        };

        let state = processor.process(State::default(), &Instruction::GetRoutes(demand));

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
}
