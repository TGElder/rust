use crate::travel_duration::*;
use crate::world::{World, WorldCell};
use commons::edge::*;
use commons::grid::Grid;
use commons::scale::*;
use commons::*;
use isometric::cell_traits::*;
use serde::{Deserialize, Serialize};
use std::default::Default;
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct RoadBuildTravelParams {
    pub max_gradient: f32,
    pub cost_at_level: f32,
    pub cost_at_max_gradient: f32,
    pub cost_on_existing_road: u64,
    pub sea_level: f32,
    pub deep_sea_level: f32,
}

impl Default for RoadBuildTravelParams {
    fn default() -> RoadBuildTravelParams {
        RoadBuildTravelParams {
            max_gradient: 0.5,
            cost_at_level: 575.0,
            cost_at_max_gradient: 925.0,
            cost_on_existing_road: 100,
            sea_level: 1.0,
            deep_sea_level: 0.67,
        }
    }
}

pub struct RoadBuildTravelDuration {
    off_road: Box<dyn TravelDuration>,
    road: Box<dyn TravelDuration>,
    parameters: RoadBuildTravelParams,
}

impl RoadBuildTravelDuration {
    pub fn from_params(p: RoadBuildTravelParams) -> RoadBuildTravelDuration {
        RoadBuildTravelDuration {
            off_road: GradientTravelDuration::boxed(
                Scale::new(
                    (-p.max_gradient, p.max_gradient),
                    (p.cost_at_level, p.cost_at_max_gradient),
                ),
                true,
            ),
            road: ConstantTravelDuration::boxed(Duration::from_millis(p.cost_on_existing_road)),
            parameters: p,
        }
    }

    fn is_inaccessible_shore(&self, world: &World, from: &V2<usize>, to: &V2<usize>) -> bool {
        let from_elevation = world.get_cell_unsafe(from).elevation;
        if from_elevation < self.parameters.deep_sea_level {
            return false;
        }
        if from_elevation > self.parameters.sea_level {
            return false;
        }

        let to_elevation = world.get_cell_unsafe(to).elevation;
        if to_elevation <= self.parameters.sea_level {
            return false;
        }

        true
    }
}

impl TravelDuration for RoadBuildTravelDuration {
    fn get_duration(&self, world: &World, from: &V2<usize>, to: &V2<usize>) -> Option<Duration> {
        match world.get_cell(from) {
            Some(WorldCell { visible: true, .. }) => (),
            _ => return None,
        };
        match world.get_cell(to) {
            Some(WorldCell { visible: true, .. }) => (),
            _ => return None,
        };
        if self.is_inaccessible_shore(world, from, to)
            || self.is_inaccessible_shore(world, to, from)
        {
            return None;
        }
        if world.is_sea(from) && world.is_sea(to) {
            return None;
        }
        if let (Some(from), Some(to)) = (world.get_cell(from), world.get_cell(to)) {
            let edge = Edge::new(from.position(), to.position());
            if from.river.corner() || to.river.corner() || (from.river.here() && to.river.here()) {
                None
            } else if world.is_road(&edge) || world.road_planned(&edge).is_some() {
                self.road
                    .get_duration(world, &from.position(), &to.position())
            } else {
                self.off_road
                    .get_duration(world, &from.position(), &to.position())
            }
        } else {
            None
        }
    }

    fn min_duration(&self) -> Duration {
        self.off_road.min_duration().min(self.road.min_duration())
    }

    fn max_duration(&self) -> Duration {
        self.off_road.max_duration().max(self.road.max_duration())
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use commons::junction::*;
    use commons::{v2, M};

    fn road_travel_duration() -> Box<dyn TravelDuration> {
        ConstantTravelDuration::boxed(Duration::from_millis(10))
    }

    fn off_road_travel_duration() -> Box<dyn TravelDuration> {
        ConstantTravelDuration::boxed(Duration::from_millis(1000))
    }

    fn road_build_travel_duration() -> RoadBuildTravelDuration {
        RoadBuildTravelDuration {
            off_road: off_road_travel_duration(),
            road: road_travel_duration(),
            parameters: RoadBuildTravelParams {
                sea_level: 1.0,
                deep_sea_level: 0.5,
                ..RoadBuildTravelParams::default()
            },
        }
    }

    #[test]
    fn defaults_to_off_road_travel_duration() {
        let mut world = World::new(
            M::from_vec(
                3,
                3,
                vec![
                    1.0, 1.0, 1.0, //
                    1.0, 1.0, 1.0, //
                    1.0, 1.0, 1.0, //
                ],
            ),
            0.5,
        );

        world.reveal_all();

        assert_eq!(
            road_build_travel_duration().get_duration(&world, &v2(0, 1), &v2(1, 1)),
            Some(off_road_travel_duration().max_duration())
        );
    }

    #[test]
    fn can_not_build_over_river_corner() {
        let mut world = World::new(
            M::from_vec(
                3,
                3,
                vec![
                    1.0, 1.0, 1.0, //
                    1.0, 1.0, 1.0, //
                    1.0, 1.0, 1.0, //
                ],
            ),
            0.5,
        );

        let mut river_1 = PositionJunction::new(v2(1, 0));
        river_1.junction.vertical.width = 1.0;
        river_1.junction.vertical.from = true;
        let mut river_2 = PositionJunction::new(v2(1, 1));
        river_2.junction.horizontal.width = 1.0;
        river_2.junction.vertical.width = 1.0;
        river_2.junction.horizontal.from = true;
        river_2.junction.vertical.to = true;
        let mut river_3 = PositionJunction::new(v2(2, 1));
        river_3.junction.horizontal.width = 1.0;
        river_3.junction.horizontal.to = true;
        world.add_river(river_1);
        world.add_river(river_2);
        world.add_river(river_3);

        world.reveal_all();

        assert_eq!(
            road_build_travel_duration().get_duration(&world, &v2(0, 1), &v2(1, 1)),
            None
        );
    }

    #[test]
    fn can_not_build_along_river() {
        let mut world = World::new(
            M::from_vec(
                3,
                3,
                vec![
                    1.0, 1.0, 1.0, //
                    1.0, 1.0, 1.0, //
                    1.0, 1.0, 1.0, //
                ],
            ),
            0.5,
        );

        let mut river_1 = PositionJunction::new(v2(1, 0));
        river_1.junction.vertical.width = 1.0;
        river_1.junction.vertical.from = true;
        let mut river_2 = PositionJunction::new(v2(1, 1));
        river_2.junction.vertical.width = 1.0;
        river_2.junction.vertical.from = true;
        river_2.junction.vertical.to = true;
        let mut river_3 = PositionJunction::new(v2(1, 2));
        river_3.junction.vertical.width = 1.0;
        river_3.junction.vertical.to = true;
        world.add_river(river_1);
        world.add_river(river_2);
        world.add_river(river_3);

        world.reveal_all();

        assert_eq!(
            road_build_travel_duration().get_duration(&world, &v2(1, 0), &v2(1, 1)),
            None
        );
    }

    #[test]
    fn can_cross_river_at_90_degrees() {
        let mut world = World::new(
            M::from_vec(
                3,
                3,
                vec![
                    1.0, 1.0, 1.0, //
                    1.0, 1.0, 1.0, //
                    1.0, 1.0, 1.0, //
                ],
            ),
            0.5,
        );

        let mut river_1 = PositionJunction::new(v2(1, 0));
        river_1.junction.vertical.width = 1.0;
        river_1.junction.vertical.from = true;
        let mut river_2 = PositionJunction::new(v2(1, 1));
        river_2.junction.vertical.width = 1.0;
        river_2.junction.vertical.from = true;
        river_2.junction.vertical.to = true;
        let mut river_3 = PositionJunction::new(v2(1, 2));
        river_3.junction.vertical.width = 1.0;
        river_3.junction.vertical.to = true;
        world.add_river(river_1);
        world.add_river(river_2);
        world.add_river(river_3);

        world.reveal_all();

        assert_eq!(
            road_build_travel_duration().get_duration(&world, &v2(0, 1), &v2(1, 1)),
            Some(off_road_travel_duration().max_duration())
        );
    }

    #[test]
    fn can_not_build_in_sea() {
        let mut world = World::new(
            M::from_vec(3, 3, vec![1.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0, 1.0, 0.0]),
            0.5,
        );

        world.reveal_all();

        assert_eq!(
            road_build_travel_duration().get_duration(&world, &v2(2, 1), &v2(3, 1)),
            None
        );
    }

    #[test]
    fn uses_road_travel_duration_for_existing_roads() {
        let mut world = World::new(
            M::from_vec(
                3,
                3,
                vec![
                    1.0, 1.0, 1.0, //
                    1.0, 1.0, 1.0, //
                    1.0, 1.0, 1.0, //
                ],
            ),
            0.5,
        );

        world.reveal_all();

        world.set_road(&Edge::new(v2(0, 0), v2(0, 1)), true);

        assert_eq!(
            road_build_travel_duration().get_duration(&world, &v2(0, 0), &v2(0, 1)),
            Some(road_travel_duration().max_duration())
        );
    }

    #[test]
    fn uses_road_travel_duration_for_planned_roads() {
        let mut world = World::new(
            M::from_vec(
                3,
                3,
                vec![
                    1.0, 1.0, 1.0, //
                    1.0, 1.0, 1.0, //
                    1.0, 1.0, 1.0, //
                ],
            ),
            0.5,
        );

        world.reveal_all();

        world.plan_road(&Edge::new(v2(0, 0), v2(0, 1)), Some(404));

        assert_eq!(
            road_build_travel_duration().get_duration(&world, &v2(0, 0), &v2(0, 1)),
            Some(road_travel_duration().max_duration())
        );
    }

    #[test]
    fn cannot_build_into_invisible() {
        let mut world = World::new(
            M::from_vec(
                3,
                3,
                vec![
                    1.0, 1.0, 1.0, //
                    1.0, 1.0, 1.0, //
                    1.0, 1.0, 1.0, //
                ],
            ),
            0.5,
        );

        world.reveal_all();
        world.mut_cell_unsafe(&v2(0, 0)).visible = false;

        assert_eq!(
            road_build_travel_duration().get_duration(&world, &v2(0, 0), &v2(1, 0)),
            None
        );
    }

    #[test]
    fn cannot_build_from_invisible() {
        let mut world = World::new(
            M::from_vec(
                3,
                3,
                vec![
                    1.0, 1.0, 1.0, //
                    1.0, 1.0, 1.0, //
                    1.0, 1.0, 1.0, //
                ],
            ),
            0.5,
        );

        world.reveal_all();
        world.mut_cell_unsafe(&v2(0, 0)).visible = false;

        assert_eq!(
            road_build_travel_duration().get_duration(&world, &v2(1, 0), &v2(0, 0)),
            None
        );
    }

    #[test]
    fn can_not_build_from_invisible() {
        let mut world = World::new(
            M::from_vec(
                3,
                3,
                vec![
                    1.0, 1.0, 1.0, //
                    1.0, 1.0, 1.0, //
                    1.0, 1.0, 1.0, //
                ],
            ),
            0.5,
        );

        world.mut_cell_unsafe(&v2(1, 0)).visible = true;

        assert_eq!(
            road_build_travel_duration().get_duration(&world, &v2(0, 0), &v2(1, 0)),
            None
        );

        world.mut_cell_unsafe(&v2(0, 0)).visible = true;

        assert_eq!(
            road_build_travel_duration().get_duration(&world, &v2(0, 0), &v2(1, 0)),
            Some(off_road_travel_duration().max_duration())
        );
    }

    #[test]
    fn can_build_on_and_off_accessible_shore() {
        let mut world = World::new(
            M::from_vec(
                3,
                3,
                vec![
                    0.0, 1.1, 1.0, //
                    1.0, 1.0, 1.0, //
                    1.0, 1.0, 1.0, //
                ],
            ),
            0.5,
        );

        world.reveal_all();

        assert!(road_build_travel_duration()
            .get_duration(&world, &v2(0, 0), &v2(1, 0))
            .is_some());
        assert!(road_build_travel_duration()
            .get_duration(&world, &v2(1, 0), &v2(0, 0))
            .is_some());
    }

    #[test]
    fn cannot_build_on_or_off_inaccessible_shore() {
        let mut world = World::new(
            M::from_vec(
                3,
                3,
                vec![
                    0.6, 1.1, 1.0, //
                    1.0, 1.0, 1.0, //
                    1.0, 1.0, 1.0, //
                ],
            ),
            0.5,
        );

        world.reveal_all();

        assert_eq!(
            road_build_travel_duration().get_duration(&world, &v2(0, 0), &v2(1, 0)),
            None
        );
        assert_eq!(
            road_build_travel_duration().get_duration(&world, &v2(1, 0), &v2(0, 0)),
            None
        );
    }

    #[test]
    fn min_duration() {
        assert_eq!(
            road_build_travel_duration().min_duration(),
            Duration::from_millis(10)
        );
    }

    #[test]
    fn max_duration() {
        assert_eq!(
            road_build_travel_duration().max_duration(),
            Duration::from_millis(1000)
        );
    }
}
