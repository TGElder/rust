mod travel_duration;

use crate::pathfinder::*;
use crate::travel_duration::*;
use crate::world::World;
use commons::edge::*;
use commons::V2;
pub use travel_duration::*;

#[derive(Debug, PartialEq)]
pub struct RoadBuilderResult {
    path: Vec<V2<usize>>,
    mode: RoadBuildMode,
}

#[derive(Debug, PartialEq)]
pub enum RoadBuildMode {
    Build,
    Demolish,
}

impl RoadBuilderResult {
    pub fn new(path: Vec<V2<usize>>, mode: RoadBuildMode) -> RoadBuilderResult {
        RoadBuilderResult { path, mode }
    }

    pub fn path(&self) -> &Vec<V2<usize>> {
        &self.path
    }

    fn edges(&self) -> Vec<Edge> {
        (0..self.path.len() - 1)
            .map(|i| Edge::new(self.path[i], self.path[i + 1]))
            .collect()
    }

    pub fn update_roads(&self, world: &mut World) {
        for edge in self.edges() {
            match self.mode {
                RoadBuildMode::Build => build_road(&edge, world),
                RoadBuildMode::Demolish => demolish_road(&edge, world),
            }
        }
    }

    pub fn update_pathfinder<T>(&self, world: &World, pathfinder: &mut Pathfinder<T>)
    where
        T: TravelDuration,
    {
        self.edges().iter().for_each(|edge| {
            pathfinder.update_from_to(world, &edge.from(), &edge.to());
            pathfinder.update_from_to(world, &edge.to(), &edge.from());
        });
    }
}

fn build_road(edge: &Edge, world: &mut World) {
    if !world.is_river_or_road(edge) {
        world.set_road(edge, true);
    }
}

fn demolish_road(edge: &Edge, world: &mut World) {
    world.set_road(edge, false);
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

        fn min_duration(&self) -> Duration {
            Duration::from_millis(1)
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
        let world = &world();
        let mut out = Pathfinder::new(world, TestDuration {});
        out.reset_edges(world);
        out
    }

    #[test]
    fn test_result_edges() {
        let result = RoadBuilderResult {
            path: vec![v2(0, 0), v2(1, 0), v2(1, 1)],
            mode: RoadBuildMode::Build,
        };
        assert_eq!(
            result.edges(),
            vec![Edge::new(v2(0, 0), v2(1, 0)), Edge::new(v2(1, 0), v2(1, 1))]
        )
    }

    #[test]
    fn test_result_mode_build() {
        let edge = Edge::new(v2(0, 0), v2(1, 0));
        let mut world = world();
        let result = RoadBuilderResult {
            path: vec![*edge.from(), *edge.to()],
            mode: RoadBuildMode::Build,
        };
        assert!(!world.is_road(&edge));
        result.update_roads(&mut world);
        assert!(world.is_road(&edge));
    }

    #[test]
    fn test_result_mode_demolish() {
        let edge = Edge::new(v2(0, 0), v2(1, 0));
        let mut world = world();
        world.set_road(&edge, true);
        let result = RoadBuilderResult {
            path: vec![*edge.from(), *edge.to()],
            mode: RoadBuildMode::Demolish,
        };
        assert!(world.is_road(&edge));
        result.update_roads(&mut world);
        assert!(!world.is_road(&edge));
    }

    #[test]
    fn test_result_update_pathfinder() {
        let mut world = world();
        let mut pathfinder = pathfinder();
        let result = RoadBuilderResult {
            path: vec![v2(0, 0), v2(1, 0)],
            mode: RoadBuildMode::Build,
        };
        assert_eq!(pathfinder.find_path(&[v2(1, 0)], &[v2(0, 0)]), None);
        result.update_roads(&mut world);
        result.update_pathfinder(&world, &mut pathfinder);
        assert_eq!(
            pathfinder.find_path(&[v2(1, 0)], &[v2(0, 0)]),
            Some(vec![v2(1, 0), v2(0, 0)])
        );
    }
}
