mod travel_duration;

use crate::avatar::*;
use crate::pathfinder::*;
use crate::travel_duration::*;
use crate::world::World;
use commons::edge::*;
use commons::V2;
pub use travel_duration::*;

#[derive(Debug, PartialEq)]
pub struct RoadBuilderResult {
    path: Vec<V2<usize>>,
    toggle: bool,
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

    pub fn update_roads(&self, world: &mut World) {
        if self.toggle {
            self.toggle_roads(world);
        } else {
            self.set_roads(world);
        }
    }

    pub fn update_pathfinder<T>(&self, world: &World, pathfinder: &mut Pathfinder<T>)
    where
        T: TravelDuration,
    {
        self.edges().iter().for_each(|edge| {
            pathfinder.update_edge(world, &edge.from(), &edge.to());
            pathfinder.update_edge(world, &edge.to(), &edge.from());
        });
    }
}

pub struct RoadBuilder<T>
where
    T: TravelDuration,
{
    pathfinder: Pathfinder<T>,
}

impl<T> RoadBuilder<T>
where
    T: TravelDuration,
{
    pub fn new(world: &World, travel_duration: T) -> RoadBuilder<T>
    where
        T: TravelDuration,
    {
        RoadBuilder {
            pathfinder: Pathfinder::new(&world, travel_duration),
        }
    }

    pub fn pathfinder(&mut self) -> &mut Pathfinder<T> {
        &mut self.pathfinder
    }

    pub fn build_forward(&self, world: &World, avatar: &AvatarState) -> Option<RoadBuilderResult> {
        if let Some(path) = avatar.forward_path() {
            let from = path[0];
            let to = path[1];
            if self
                .pathfinder
                .travel_duration()
                .get_duration(&world, &from, &to)
                .is_some()
            {
                let result = RoadBuilderResult { toggle: true, path };
                return Some(result);
            }
        }
        None
    }

    pub fn auto_build_road(
        &self,
        avatar: &AvatarState,
        to: &V2<usize>,
    ) -> Option<RoadBuilderResult> {
        if let AvatarState::Stationary { position: from, .. } = avatar {
            if let Some(path) = self.pathfinder.find_path(from, to) {
                let result = RoadBuilderResult {
                    toggle: false,
                    path,
                };
                return Some(result);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use commons::{v2, M};
    use std::time::Duration;

    struct TestDuration {}

    impl TravelDuration for TestDuration {
        fn get_duration(
            &self,
            world: &World,
            from: &V2<usize>,
            to: &V2<usize>,
        ) -> Option<Duration> {
            if to.x > from.x || to.y > from.y || world.is_road(&Edge::new(*from, *to)) {
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

    fn pathfinder() -> Pathfinder<TestDuration> {
        Pathfinder::new(&world(), TestDuration {})
    }

    #[test]
    fn test_result_edges() {
        let result = RoadBuilderResult {
            toggle: true,
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
            toggle: true,
            path: vec![*edge.from(), *edge.to()],
        };
        assert!(!world.is_road(&edge));
        result.update_roads(&mut world);
        assert!(world.is_road(&edge));
        result.update_roads(&mut world);
        assert!(!world.is_road(&edge));
    }

    #[test]
    fn test_result_set_roads() {
        let edge = Edge::new(v2(0, 0), v2(1, 0));
        let mut world = world();
        let result = RoadBuilderResult {
            toggle: false,
            path: vec![*edge.from(), *edge.to()],
        };
        assert!(!world.is_road(&edge));
        result.update_roads(&mut world);
        assert!(world.is_road(&edge));
        result.update_roads(&mut world);
        assert!(world.is_road(&edge));
    }

    #[test]
    fn test_result_update_pathfinder() {
        let mut world = world();
        let mut pathfinder = pathfinder();
        let result = RoadBuilderResult {
            toggle: true,
            path: vec![v2(0, 0), v2(1, 0)],
        };
        assert_eq!(pathfinder.find_path(&v2(1, 0), &v2(0, 0)), None);
        result.update_roads(&mut world);
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
        let avatar = AvatarState::Stationary {
            position: v2(0, 0),
            rotation: Rotation::Right,
        };
        let mut road_builder = RoadBuilder { pathfinder };
        assert_eq!(
            road_builder.pathfinder.find_path(&v2(1, 0), &v2(0, 0)),
            None
        );
        assert!(!world.is_road(&Edge::new(v2(0, 0), v2(1, 0))));
        let result = road_builder.auto_build_road(&avatar, &v2(1, 0)).unwrap();
        assert_eq!(
            result,
            RoadBuilderResult {
                toggle: false,
                path: vec![v2(0, 0), v2(1, 0)]
            }
        );
        result.update_roads(&mut world);
        result.update_pathfinder(&world, road_builder.pathfinder());
        assert_eq!(
            road_builder.pathfinder.find_path(&v2(1, 0), &v2(0, 0)),
            Some(vec![v2(1, 0), v2(0, 0)])
        );
        assert!(world.is_road(&Edge::new(v2(0, 0), v2(1, 0))));
    }

    #[test]
    fn test_auto_build_road_impossible() {
        let pathfinder = pathfinder();
        let road_builder = RoadBuilder { pathfinder };
        let avatar = AvatarState::Stationary {
            position: v2(1, 0),
            rotation: Rotation::Left,
        };
        assert_eq!(road_builder.auto_build_road(&avatar, &v2(1, 0)), None);
    }

    #[test]
    fn test_build_forward() {
        let mut world = world();
        let pathfinder = pathfinder();
        let avatar = AvatarState::Stationary {
            position: v2(0, 0),
            rotation: Rotation::Right,
        };
        let mut road_builder = RoadBuilder { pathfinder };
        assert_eq!(
            road_builder.pathfinder.find_path(&v2(1, 0), &v2(0, 0)),
            None
        );
        assert!(!world.is_road(&Edge::new(v2(0, 0), v2(1, 0))));
        let result = road_builder.build_forward(&world, &avatar).unwrap();
        assert_eq!(
            result,
            RoadBuilderResult {
                toggle: true,
                path: vec![v2(0, 0), v2(1, 0)]
            }
        );
        result.update_roads(&mut world);
        result.update_pathfinder(&world, road_builder.pathfinder());
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
        let pathfinder = Pathfinder::new(&world, TestDuration {});
        let avatar = AvatarState::Stationary {
            position: v2(0, 0),
            rotation: Rotation::Right,
        };
        let mut road_builder = RoadBuilder { pathfinder };
        assert_eq!(
            road_builder.pathfinder.find_path(&v2(1, 0), &v2(0, 0)),
            Some(vec![v2(1, 0), v2(0, 0)])
        );
        assert!(world.is_road(&Edge::new(v2(0, 0), v2(1, 0))));
        let result = road_builder.build_forward(&world, &avatar).unwrap();
        assert_eq!(
            result,
            RoadBuilderResult {
                toggle: true,
                path: vec![v2(0, 0), v2(1, 0)]
            }
        );
        result.update_roads(&mut world);
        result.update_pathfinder(&world, road_builder.pathfinder());
        assert_eq!(
            road_builder.pathfinder.find_path(&v2(1, 0), &v2(0, 0)),
            None
        );
        assert!(!world.is_road(&Edge::new(v2(0, 0), v2(1, 0))));
    }

    #[test]
    fn test_build_forward_impossible() {
        let world = world();
        let pathfinder = pathfinder();
        let avatar = AvatarState::Stationary {
            position: v2(1, 0),
            rotation: Rotation::Left,
        };
        let road_builder = RoadBuilder { pathfinder };
        assert_eq!(road_builder.build_forward(&world, &avatar), None);
    }

}
