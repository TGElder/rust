use super::*;
use crate::pathfinder::traits::{ClosestTargetResult, ClosestTargets};
use crate::route::{Route, RouteKey};
use crate::simulation_2::game_event_consumers::target_set;

pub struct DemandToRoutes<P>
where
    P: ClosestTargets,
{
    pathfinder: Arc<RwLock<P>>,
}

impl<P> Processor for DemandToRoutes<P>
where
    P: ClosestTargets,
{
    fn process(&mut self, mut state: State, instruction: &Instruction) -> State {
        let demand = match instruction {
            Instruction::Demand(demand) => *demand,
            _ => return state,
        };
        for (key, route) in routes(&demand, self.closest_targets(&demand)) {
            state.instructions.push(Instruction::Route { key, route });
        }
        state
    }
}

impl<P> DemandToRoutes<P>
where
    P: ClosestTargets,
{
    pub fn new(pathfinder: &Arc<RwLock<P>>) -> DemandToRoutes<P> {
        DemandToRoutes {
            pathfinder: pathfinder.clone(),
        }
    }

    fn closest_targets(&self, demand: &Demand) -> Vec<ClosestTargetResult> {
        let positions = [demand.position];
        let target_set = target_set(demand.resource);
        let sources = demand.sources;
        self.pathfinder
            .read()
            .unwrap()
            .closest_targets(&positions, &target_set, sources)
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

    use crate::world::Resource;
    use commons::v2;
    use std::time::Duration;

    #[test]
    fn test() {
        struct MockPathfinder {}

        impl ClosestTargets for MockPathfinder {
            fn init_targets(&mut self, _: String) {}

            fn load_target(&mut self, _: &str, _: &V2<usize>, _: bool) {}

            fn closest_targets(
                &self,
                position: &[V2<usize>],
                target_set: &str,
                n_closest: usize,
            ) -> Vec<ClosestTargetResult> {
                assert_eq!(position, &[v2(1, 3)]);
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

        let pathfinder = Arc::new(RwLock::new(MockPathfinder {}));
        let mut processor = DemandToRoutes::new(&pathfinder);
        let demand = Demand {
            position: v2(1, 3),
            resource: Resource::Coal,
            sources: 2,
            quantity: 3,
        };

        let state = processor.process(State::default(), &Instruction::Demand(demand));

        assert_eq!(
            state.instructions,
            vec![
                Instruction::Route {
                    key: RouteKey {
                        settlement: v2(1, 3),
                        resource: Resource::Coal,
                        destination: v2(1, 5),
                    },
                    route: Route {
                        path: vec![v2(1, 3), v2(1, 4), v2(1, 5)],
                        start_micros: 0,
                        duration: Duration::from_secs(2),
                        traffic: 3,
                    }
                },
                Instruction::Route {
                    key: RouteKey {
                        settlement: v2(1, 3),
                        resource: Resource::Coal,
                        destination: v2(5, 3),
                    },
                    route: Route {
                        path: vec![v2(1, 3), v2(2, 3), v2(3, 3), v2(4, 3), v2(5, 3)],
                        start_micros: 0,
                        duration: Duration::from_secs(4),
                        traffic: 3,
                    }
                },
            ]
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
                position: &[V2<usize>],
                target_set: &str,
                n_closest: usize,
            ) -> Vec<ClosestTargetResult> {
                assert_eq!(position, &[v2(1, 3)]);
                assert_eq!(target_set, "resource-coal");
                assert_eq!(n_closest, 2);
                vec![]
            }
        }

        let pathfinder = Arc::new(RwLock::new(MockPathfinder {}));
        let mut processor = DemandToRoutes::new(&pathfinder);
        let demand = Demand {
            position: v2(1, 3),
            resource: Resource::Coal,
            sources: 2,
            quantity: 3,
        };

        let state = processor.process(State::default(), &Instruction::Demand(demand));

        assert_eq!(state.instructions, vec![]);
    }
}
