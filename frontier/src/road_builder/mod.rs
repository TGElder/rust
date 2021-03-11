mod travel_duration;

use std::collections::HashSet;
use std::iter::once;

use crate::world::World;
use commons::{edge::*, V2};
pub use travel_duration::*;

#[derive(Debug, PartialEq)]
pub struct RoadBuilderResult {
    edges: Vec<Edge>,
    mode: RoadBuildMode,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RoadBuildMode {
    Build,
    Demolish,
}

impl RoadBuilderResult {
    pub fn new(edges: Vec<Edge>, mode: RoadBuildMode) -> RoadBuilderResult {
        RoadBuilderResult { edges, mode }
    }

    pub fn edges(&self) -> &[Edge] {
        &self.edges
    }

    pub fn positions(&self) -> HashSet<V2<usize>> {
        self.edges()
            .iter()
            .flat_map(|edge| once(*edge.from()).chain(once(*edge.to())))
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

    #[test]
    fn test_positions() {
        let result = RoadBuilderResult {
            edges: vec![Edge::new(v2(0, 1), v2(0, 2)), Edge::new(v2(0, 2), v2(0, 3))],
            mode: RoadBuildMode::Build,
        };
        assert_eq!(
            result.positions(),
            hashset! {
                v2(0, 1),
                v2(0, 2),
                v2(0, 3),
            }
        )
    }

    #[test]
    fn test_mode_build() {
        let edge = Edge::new(v2(0, 0), v2(1, 0));
        let mut world = world();
        let result = RoadBuilderResult {
            edges: vec![edge],
            mode: RoadBuildMode::Build,
        };
        assert!(!world.is_road(&edge));
        result.update_roads(&mut world);
        assert!(world.is_road(&edge));
    }

    #[test]
    fn test_mode_demolish() {
        let edge = Edge::new(v2(0, 0), v2(1, 0));
        let mut world = world();
        world.set_road(&edge, true);
        let result = RoadBuilderResult {
            edges: vec![edge],
            mode: RoadBuildMode::Demolish,
        };
        assert!(world.is_road(&edge));
        result.update_roads(&mut world);
        assert!(!world.is_road(&edge));
    }
}
