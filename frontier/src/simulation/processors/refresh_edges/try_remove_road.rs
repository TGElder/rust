use commons::edge::Edge;
use commons::futures::FutureExt;

use super::*;
use crate::pathfinder::traits::UpdateEdge;

use crate::game::traits::{BuildRoad, HasWorld};

const ROAD_THRESHOLD: usize = 0;

pub async fn try_remove_road<G, R, P>(
    state: &mut State,
    world: &mut G,
    build_road_tx: &FnSender<R>,
    pathfinder: &Arc<RwLock<P>>,
    traffic: &EdgeTrafficSummary,
) where
    G: HasWorld + Send,
    R: BuildRoad + Send,
    P: UpdateEdge + Send + Sync + 'static,
{
    if get_traffic(&traffic.routes) > ROAD_THRESHOLD {
        return;
    }
    match traffic.road_status {
        RoadStatus::Planned(..) => (),
        RoadStatus::Built => (),
        _ => return,
    };

    state.build_queue.remove(&BuildKey::Road(traffic.edge));
    remove_plan(world, traffic.edge);
    update_edge(world, pathfinder, traffic.edge);
    send_remove_road(build_road_tx, traffic.edge);
}

fn remove_plan<G>(game: &mut G, edge: Edge)
where
    G: HasWorld + Send,
{
    game.world_mut().plan_road(&edge, None);
}

fn send_remove_road<R>(build_road_tx: &FnSender<R>, edge: Edge)
where
    R: BuildRoad + Send,
{
    build_road_tx.send_future(move |build_road| remove_road(build_road, edge).boxed());
}

async fn remove_road<R>(build_road: &mut R, edge: Edge)
where
    R: BuildRoad + Send,
{
    build_road.remove_road(&edge).await;
}

fn update_edge<G, P>(game: &mut G, pathfinder: &Arc<RwLock<P>>, edge: Edge)
where
    G: HasWorld + Send,
    P: UpdateEdge,
{
    pathfinder
        .write()
        .unwrap()
        .update_edge(&game.world(), &edge);
}

fn get_traffic(routes: &[EdgeRouteSummary]) -> usize {
    routes.iter().map(|route| route.traffic).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::world::World;
    use commons::async_trait::async_trait;
    use commons::edge::Edge;
    use commons::fn_sender::FnThread;
    use commons::futures::executor::block_on;
    use commons::{v2, M};
    use std::default::Default;

    fn world() -> World {
        World::new(M::zeros(4, 4), 0.5)
    }

    struct MockBuildRoads {
        removed_roads: Vec<Edge>,
    }

    impl Default for MockBuildRoads {
        fn default() -> MockBuildRoads {
            MockBuildRoads {
                removed_roads: vec![],
            }
        }
    }

    #[async_trait]
    impl BuildRoad for MockBuildRoads {
        async fn add_road(&mut self, _: &Edge) {}

        async fn remove_road(&mut self, edge: &Edge) {
            self.removed_roads.push(*edge);
        }
    }

    #[test]
    fn should_remove_road_if_status_is_built_and_traffic_under_threshold() {
        // Given
        let edge = Edge::new(v2(1, 2), v2(1, 3));

        let mut state = State::default();
        state.build_queue.insert(BuildInstruction {
            what: Build::Road(edge),
            when: 10,
        });

        let mut world = world();
        world.plan_road(&edge, Some(123));

        let build_road = FnThread::new(MockBuildRoads::default());

        let pathfinder = Arc::new(RwLock::new(vec![]));

        // When
        block_on(try_remove_road(
            &mut state,
            &mut world,
            build_road.tx(),
            &pathfinder,
            &EdgeTrafficSummary {
                edge,
                road_status: RoadStatus::Built,
                routes: vec![EdgeRouteSummary {
                    traffic: 0,
                    first_visit: 0,
                }],
            },
        ));

        // Then
        assert_eq!(state.build_queue, BuildQueue::default());
        assert_eq!(build_road.join().removed_roads, vec![edge]);
        assert_eq!(world.road_planned(&edge), None);
        assert_eq!(*pathfinder.read().unwrap(), vec![edge]);
    }

    #[test]
    fn should_remove_road_if_status_is_planned_and_traffic_under_threshold() {
        // Given
        let edge = Edge::new(v2(1, 2), v2(1, 3));

        let mut state = State::default();
        state.build_queue.insert(BuildInstruction {
            what: Build::Road(edge),
            when: 10,
        });

        let mut world = world();
        world.plan_road(&edge, Some(123));

        let build_road = FnThread::new(MockBuildRoads::default());

        let pathfinder = Arc::new(RwLock::new(vec![]));

        // When
        block_on(try_remove_road(
            &mut state,
            &mut world,
            build_road.tx(),
            &pathfinder,
            &EdgeTrafficSummary {
                edge,
                road_status: RoadStatus::Planned(123),
                routes: vec![EdgeRouteSummary {
                    traffic: 0,
                    first_visit: 0,
                }],
            },
        ));

        // Then
        assert_eq!(state.build_queue, BuildQueue::default());
        assert_eq!(build_road.join().removed_roads, vec![edge]);
        assert_eq!(world.road_planned(&edge), None);
        assert_eq!(*pathfinder.read().unwrap(), vec![edge]);
    }

    #[test]
    fn should_not_remove_road_if_traffic_exceeds_threshold() {
        // Given
        let edge = Edge::new(v2(1, 2), v2(1, 3));

        let mut build_queue = BuildQueue::default();
        build_queue.insert(BuildInstruction {
            what: Build::Road(edge),
            when: 10,
        });
        let mut state = State::default();
        state.build_queue = build_queue.clone();

        let mut world = world();
        world.plan_road(&edge, Some(123));

        let build_road = FnThread::new(MockBuildRoads::default());

        let pathfinder = Arc::new(RwLock::new(vec![]));

        // When
        block_on(try_remove_road(
            &mut state,
            &mut world,
            build_road.tx(),
            &pathfinder,
            &EdgeTrafficSummary {
                edge,
                road_status: RoadStatus::Suitable,
                routes: vec![EdgeRouteSummary {
                    traffic: ROAD_THRESHOLD + 1,
                    first_visit: 0,
                }],
            },
        ));

        // Then
        assert_eq!(state.build_queue, build_queue);
        assert_eq!(build_road.join().removed_roads, vec![]);
        assert_eq!(world.road_planned(&edge), Some(123));
        assert_eq!(*pathfinder.read().unwrap(), vec![]);
    }

    #[test]
    fn should_not_remove_road_if_status_is_suitable() {
        // Given
        let edge = Edge::new(v2(1, 2), v2(1, 3));

        let mut world = world();
        let build_road = FnThread::new(MockBuildRoads::default());
        let pathfinder = Arc::new(RwLock::new(vec![]));

        // When
        block_on(try_remove_road(
            &mut State::default(),
            &mut world,
            build_road.tx(),
            &pathfinder,
            &EdgeTrafficSummary {
                edge,
                road_status: RoadStatus::Suitable,
                routes: vec![EdgeRouteSummary {
                    traffic: 0,
                    first_visit: 0,
                }],
            },
        ));

        // Then
        assert_eq!(build_road.join().removed_roads, vec![]);
        assert_eq!(*pathfinder.read().unwrap(), vec![]);
    }

    #[test]
    fn should_not_remove_road_if_status_is_unsuitable() {
        // Given
        let edge = Edge::new(v2(1, 2), v2(1, 3));

        let mut world = world();
        let build_road = FnThread::new(MockBuildRoads::default());
        let pathfinder = Arc::new(RwLock::new(vec![]));

        // When
        block_on(try_remove_road(
            &mut State::default(),
            &mut world,
            build_road.tx(),
            &pathfinder,
            &EdgeTrafficSummary {
                edge,
                road_status: RoadStatus::Unsuitable,
                routes: vec![EdgeRouteSummary {
                    traffic: 0,
                    first_visit: 0,
                }],
            },
        ));

        // Then
        assert_eq!(build_road.join().removed_roads, vec![]);
        assert_eq!(*pathfinder.read().unwrap(), vec![]);
    }
}
