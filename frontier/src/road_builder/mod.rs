mod travel_duration;

use crate::avatar::*;
use crate::pathfinder::*;
use crate::travel_duration::*;
use crate::world::World;
use commons::edge::*;
use commons::scale::*;
use commons::V2;
use std::time::Duration;
use travel_duration::*;

#[derive(Debug, PartialEq)]
pub struct RoadBuilderResult {
    path: Vec<V2<usize>>,
}

impl RoadBuilderResult {
    pub fn path(&self) -> &Vec<V2<usize>> {
        &self.path
    }

    fn edges(&self) -> Vec<Edge> {
        (0..self.path.len() - 1)
            .map(|i| Edge::new(self.path[i], self.path[i + 1]))
            .collect()
    }

    fn toggle_roads(&self, world: &mut World) {
        for edge in self.edges() {
            world.toggle_road(&edge);
        }
    }

    fn set_roads(&self, world: &mut World) {
        for edge in self.edges() {
            if !world.is_river_or_road(&edge) {
                world.toggle_road(&edge);
            }
        }
    }

    pub fn update_pathfinder(&self, world: &World, pathfinder: &mut Pathfinder) {
        self.edges().iter().for_each(|edge| {
            pathfinder.update_edge(world, &edge.from(), &edge.to());
            pathfinder.update_edge(world, &edge.to(), &edge.from());
        });
    }
}

pub struct RoadBuilder {
    pathfinder: Pathfinder,
}

impl RoadBuilder {
    pub fn new(world: &World) -> RoadBuilder {
        RoadBuilder {
            pathfinder: Pathfinder::new(
                &world,
                Box::new(AutoRoadTravelDuration::new(
                    GradientTravelDuration::boxed(Scale::new((-0.3, 0.3), (575.0, 925.0)), true),
                    ConstantTravelDuration::boxed(Duration::from_millis(100)),
                )),
            ),
        }
    }

    pub fn pathfinder(&mut self) -> &mut Pathfinder {
        &mut self.pathfinder
    }

    pub fn build_forward(
        &mut self,
        world: &mut World,
        avatar: &Avatar,
    ) -> Option<RoadBuilderResult> {
        if let Some(path) = avatar.forward_path() {
            let from = path[0];
            let to = path[1];
            if let Some(_) = self
                .pathfinder
                .travel_duration()
                .get_duration(&world, &from, &to)
            {
                let result = RoadBuilderResult { path };
                result.toggle_roads(world);
                result.update_pathfinder(world, &mut self.pathfinder);
                return Some(result);
            }
        }
        return None;
    }

    pub fn auto_build_road(
        &mut self,
        world: &mut World,
        avatar: &Avatar,
        to: &V2<usize>,
    ) -> Option<RoadBuilderResult> {
        if let Some(AvatarState::Stationary { position: from, .. }) = avatar.state() {
            if let Some(path) = self.pathfinder.find_path(from, to) {
                let result = RoadBuilderResult { path };
                result.set_roads(world);
                result.update_pathfinder(world, &mut self.pathfinder);
                return Some(result);
            }
        }
        return None;
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use commons::{v2, M};

    struct TestDuration {}

    impl TravelDuration for TestDuration {
        fn get_duration(
            &self,
            world: &World,
            from: &V2<usize>,
            to: &V2<usize>,
        ) -> Option<Duration> {
            if to.x > from.x || to.y > from.y {
                Some(Duration::from_millis(1))
            } else if world.is_road(&Edge::new(*from, *to)) {
                Some(Duration::from_millis(1))
            } else {
                None
            }
        }

        fn max_duration(&self) -> Duration {
            Duration::from_millis(1)
        }
    }

    #[rustfmt::skip]
    fn world() -> World {
        World::new(
            M::from_vec(3, 3, vec![
                1.0, 1.0, 1.0,
                1.0, 1.0, 1.0,
                1.0, 1.0, 1.0,
            ]),
            0.5,
        )
    }

    fn pathfinder() -> Pathfinder {
        Pathfinder::new(&world(), Box::new(TestDuration {}))
    }

    #[test]
    fn test_result_edges() {
        let result = RoadBuilderResult {
            path: vec![v2(0, 0), v2(1, 0), v2(1, 1)],
        };
        assert_eq!(
            result.edges(),
            vec![Edge::new(v2(0, 0), v2(1, 0)), Edge::new(v2(1, 0), v2(1, 1))]
        )
    }

    #[test]
    fn test_result_toggle_roads() {
        let edge = Edge::new(v2(0, 0), v2(1, 0));
        let mut world = world();
        let result = RoadBuilderResult {
            path: vec![*edge.from(), *edge.to()],
        };
        assert!(!world.is_road(&edge));
        result.toggle_roads(&mut world);
        assert!(world.is_road(&edge));
        result.toggle_roads(&mut world);
        assert!(!world.is_road(&edge));
    }

    #[test]
    fn test_result_set_roads() {
        let edge = Edge::new(v2(0, 0), v2(1, 0));
        let mut world = world();
        let result = RoadBuilderResult {
            path: vec![*edge.from(), *edge.to()],
        };
        assert!(!world.is_road(&edge));
        result.set_roads(&mut world);
        assert!(world.is_road(&edge));
        result.set_roads(&mut world);
        assert!(world.is_road(&edge));
    }

    #[test]
    fn test_result_update_pathfinder() {
        let mut world = world();
        let mut pathfinder = pathfinder();
        let result = RoadBuilderResult {
            path: vec![v2(0, 0), v2(1, 0)],
        };
        assert_eq!(pathfinder.find_path(&v2(1, 0), &v2(0, 0)), None);
        result.toggle_roads(&mut world);
        result.update_pathfinder(&world, &mut pathfinder);
        assert_eq!(
            pathfinder.find_path(&v2(1, 0), &v2(0, 0)),
            Some(vec![v2(1, 0), v2(0, 0)])
        );
    }

    #[test]
    fn test_auto_build_road() {
        let mut world = world();
        let pathfinder = pathfinder();
        let mut avatar = Avatar::new(0.0);
        let mut road_builder = RoadBuilder { pathfinder };
        avatar.reposition(v2(0, 0), Rotation::Right);
        assert_eq!(
            road_builder.pathfinder.find_path(&v2(1, 0), &v2(0, 0)),
            None
        );
        assert!(!world.is_road(&Edge::new(v2(0, 0), v2(1, 0))));
        assert_eq!(
            road_builder.auto_build_road(&mut world, &avatar, &v2(1, 0)),
            Some(RoadBuilderResult {
                path: vec![v2(0, 0), v2(1, 0)]
            })
        );
        assert_eq!(
            road_builder.pathfinder.find_path(&v2(1, 0), &v2(0, 0)),
            Some(vec![v2(1, 0), v2(0, 0)])
        );
        assert!(world.is_road(&Edge::new(v2(0, 0), v2(1, 0))));
    }

    #[test]
    fn test_auto_build_road_impossible() {
        let mut world = world();
        let pathfinder = pathfinder();
        let mut avatar = Avatar::new(0.0);
        let mut road_builder = RoadBuilder { pathfinder };
        avatar.reposition(v2(1, 0), Rotation::Left);
        assert_eq!(
            road_builder.auto_build_road(&mut world, &avatar, &v2(1, 0)),
            None
        );
    }

    #[test]
    fn test_build_forward() {
        let mut world = world();
        let pathfinder = pathfinder();
        let mut avatar = Avatar::new(0.0);
        let mut road_builder = RoadBuilder { pathfinder };
        avatar.reposition(v2(0, 0), Rotation::Right);
        assert_eq!(
            road_builder.pathfinder.find_path(&v2(1, 0), &v2(0, 0)),
            None
        );
        assert!(!world.is_road(&Edge::new(v2(0, 0), v2(1, 0))));
        assert_eq!(
            road_builder.build_forward(&mut world, &avatar),
            Some(RoadBuilderResult {
                path: vec![v2(0, 0), v2(1, 0)]
            })
        );
        assert_eq!(
            road_builder.pathfinder.find_path(&v2(1, 0), &v2(0, 0)),
            Some(vec![v2(1, 0), v2(0, 0)])
        );
        assert!(world.is_road(&Edge::new(v2(0, 0), v2(1, 0))));
    }

    #[test]
    fn test_build_forward_delete_road() {
        let mut world = world();
        world.toggle_road(&Edge::new(v2(0, 0), v2(1, 0)));
        let pathfinder = Pathfinder::new(&world, Box::new(TestDuration {}));
        let mut avatar = Avatar::new(0.0);
        let mut road_builder = RoadBuilder { pathfinder };
        avatar.reposition(v2(0, 0), Rotation::Right);
        assert_eq!(
            road_builder.pathfinder.find_path(&v2(1, 0), &v2(0, 0)),
            Some(vec![v2(1, 0), v2(0, 0)])
        );
        assert!(world.is_road(&Edge::new(v2(0, 0), v2(1, 0))));
        assert_eq!(
            road_builder.build_forward(&mut world, &avatar),
            Some(RoadBuilderResult {
                path: vec![v2(0, 0), v2(1, 0)]
            })
        );
        assert_eq!(
            road_builder.pathfinder.find_path(&v2(1, 0), &v2(0, 0)),
            None
        );
        assert!(!world.is_road(&Edge::new(v2(0, 0), v2(1, 0))));
    }

    #[test]
    fn test_build_forward_impossible() {
        let mut world = world();
        let pathfinder = pathfinder();
        let mut avatar = Avatar::new(0.0);
        let mut road_builder = RoadBuilder { pathfinder };
        avatar.reposition(v2(1, 0), Rotation::Left);
        assert_eq!(road_builder.build_forward(&mut world, &avatar), None);
    }

}
