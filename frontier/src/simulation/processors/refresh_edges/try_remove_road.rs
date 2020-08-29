use super::*;
use crate::pathfinder::traits::UpdateEdge;

use crate::game::traits::{BuildRoad, HasWorld};

const ROAD_THRESHOLD: usize = 0;

pub fn try_remove_road<G, P>(
    state: &mut State,
    game: &mut G,
    pathfinder: &Arc<RwLock<P>>,
    traffic: &EdgeTrafficSummary,
) where
    G: BuildRoad + HasWorld,
    P: UpdateEdge,
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
    game.remove_road(&traffic.edge);
    game.world_mut().plan_road(&traffic.edge, None);
    pathfinder
        .write()
        .unwrap()
        .update_edge(&game.world(), &traffic.edge);
}

fn get_traffic(routes: &[EdgeRouteSummary]) -> usize {
    routes.iter().map(|route| route.traffic).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::world::World;
    use commons::edge::Edge;
    use commons::{v2, M};
    use std::default::Default;

    fn world() -> World {
        World::new(M::zeros(4, 4), 0.5)
    }

    struct MockGame {
        world: World,
        removed_roads: Vec<Edge>,
    }

    impl Default for MockGame {
        fn default() -> MockGame {
            MockGame {
                world: world(),
                removed_roads: vec![],
            }
        }
    }

    impl BuildRoad for MockGame {
        fn add_road(&mut self, _: &Edge) {}

        fn remove_road(&mut self, edge: &Edge) {
            self.removed_roads.push(*edge);
        }
    }

    impl HasWorld for MockGame {
        fn world(&self) -> &World {
            &self.world
        }

        fn world_mut(&mut self) -> &mut World {
            &mut self.world
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

        let mut game = MockGame::default();
        game.world.plan_road(&edge, Some(123));
        let pathfinder = Arc::new(RwLock::new(vec![]));

        // When
        try_remove_road(
            &mut state,
            &mut game,
            &pathfinder,
            &EdgeTrafficSummary {
                edge,
                road_status: RoadStatus::Built,
                routes: vec![EdgeRouteSummary {
                    traffic: 0,
                    first_visit: 0,
                }],
            },
        );

        // Then
        assert_eq!(state.build_queue, BuildQueue::default());
        assert_eq!(game.removed_roads, vec![edge]);
        assert_eq!(game.world().road_planned(&edge), None);
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

        let mut game = MockGame::default();
        game.world.plan_road(&edge, Some(123));
        let pathfinder = Arc::new(RwLock::new(vec![]));

        // When
        try_remove_road(
            &mut state,
            &mut game,
            &pathfinder,
            &EdgeTrafficSummary {
                edge,
                road_status: RoadStatus::Planned(123),
                routes: vec![EdgeRouteSummary {
                    traffic: 0,
                    first_visit: 0,
                }],
            },
        );

        // Then
        assert_eq!(state.build_queue, BuildQueue::default());
        assert_eq!(game.removed_roads, vec![edge]);
        assert_eq!(game.world().road_planned(&edge), None);
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

        let mut game = MockGame::default();
        game.world.plan_road(&edge, Some(123));
        let pathfinder = Arc::new(RwLock::new(vec![]));

        // When
        try_remove_road(
            &mut state,
            &mut game,
            &pathfinder,
            &EdgeTrafficSummary {
                edge,
                road_status: RoadStatus::Suitable,
                routes: vec![EdgeRouteSummary {
                    traffic: ROAD_THRESHOLD + 1,
                    first_visit: 0,
                }],
            },
        );

        // Then
        assert_eq!(state.build_queue, build_queue);
        assert_eq!(game.removed_roads, vec![]);
        assert_eq!(game.world().road_planned(&edge), Some(123));
        assert_eq!(*pathfinder.read().unwrap(), vec![]);
    }

    #[test]
    fn should_not_remove_road_if_status_is_suitable() {
        // Given
        let edge = Edge::new(v2(1, 2), v2(1, 3));
        let mut game = MockGame::default();
        let pathfinder = Arc::new(RwLock::new(vec![]));

        // When
        try_remove_road(
            &mut State::default(),
            &mut game,
            &pathfinder,
            &EdgeTrafficSummary {
                edge,
                road_status: RoadStatus::Suitable,
                routes: vec![EdgeRouteSummary {
                    traffic: 0,
                    first_visit: 0,
                }],
            },
        );

        // Then
        assert_eq!(game.removed_roads, vec![]);
        assert_eq!(*pathfinder.read().unwrap(), vec![]);
    }

    #[test]
    fn should_not_remove_road_if_status_is_unsuitable() {
        // Given
        let edge = Edge::new(v2(1, 2), v2(1, 3));
        let mut game = MockGame::default();
        let pathfinder = Arc::new(RwLock::new(vec![]));

        // When
        try_remove_road(
            &mut State::default(),
            &mut game,
            &pathfinder,
            &EdgeTrafficSummary {
                edge,
                road_status: RoadStatus::Unsuitable,
                routes: vec![EdgeRouteSummary {
                    traffic: 0,
                    first_visit: 0,
                }],
            },
        );

        // Then
        assert_eq!(game.removed_roads, vec![]);
        assert_eq!(*pathfinder.read().unwrap(), vec![]);
    }
}
