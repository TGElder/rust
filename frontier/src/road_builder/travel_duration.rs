use crate::travel_duration::*;
use crate::world::{World, WorldCell};
use commons::edge::*;
use commons::scale::*;
use commons::*;
use isometric::cell_traits::*;
use serde::{Deserialize, Serialize};
use std::default::Default;
use std::time::Duration;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct AutoRoadTravelParams {
    max_gradient: f32,
    cost_at_level: f32,
    cost_at_max_gradient: f32,
    cost_on_existing_road: u64,
}

impl Default for AutoRoadTravelParams {
    fn default() -> AutoRoadTravelParams {
        AutoRoadTravelParams {
            max_gradient: 0.5,
            cost_at_level: 575.0,
            cost_at_max_gradient: 925.0,
            cost_on_existing_road: 100,
        }
    }
}

pub struct AutoRoadTravelDuration {
    off_road: Box<dyn TravelDuration>,
    road: Box<dyn TravelDuration>,
}

impl AutoRoadTravelDuration {
    pub fn new(
        off_road: Box<dyn TravelDuration>,
        road: Box<dyn TravelDuration>,
    ) -> AutoRoadTravelDuration {
        AutoRoadTravelDuration { off_road, road }
    }

    pub fn from_params(params: &AutoRoadTravelParams) -> AutoRoadTravelDuration {
        AutoRoadTravelDuration::new(
            GradientTravelDuration::boxed(
                Scale::new(
                    (-params.max_gradient, params.max_gradient),
                    (params.cost_at_level, params.cost_at_max_gradient),
                ),
                true,
            ),
            ConstantTravelDuration::boxed(Duration::from_millis(params.cost_on_existing_road)),
        )
    }
}

impl TravelDuration for AutoRoadTravelDuration {
    fn get_duration(&self, world: &World, from: &V2<usize>, to: &V2<usize>) -> Option<Duration> {
        match world.get_cell(from) {
            Some(WorldCell { visible: true, .. }) => (),
            _ => return None,
        };
        match world.get_cell(to) {
            Some(WorldCell { visible: true, .. }) => (),
            _ => return None,
        };
        if world.is_sea(from) && world.is_sea(to) {
            return None;
        }
        if let (Some(from), Some(to)) = (world.get_cell(from), world.get_cell(to)) {
            if from.river.corner() || to.river.corner() || (from.river.here() && to.river.here()) {
                None
            } else if world.is_road(&Edge::new(from.position(), to.position())) {
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

    fn auto_road_travel_duration() -> AutoRoadTravelDuration {
        AutoRoadTravelDuration::new(off_road_travel_duration(), road_travel_duration())
    }

    #[rustfmt::skip]
    #[test]
    fn defaults_to_off_road_travel_duration() {
        let mut world = World::new(
            M::from_vec(3, 3, vec![
                1.0, 1.0, 1.0,
                1.0, 1.0, 1.0,
                1.0, 1.0, 1.0,
            ]),
            0.5,
        );

        world.reveal_all();

        assert_eq!(auto_road_travel_duration().get_duration(&world, &v2(0, 1), &v2(1, 1)), Some(off_road_travel_duration().max_duration()));
    }

    #[rustfmt::skip]
    #[test]
    fn can_not_build_over_river_corner() {
        let mut world = World::new(
            M::from_vec(3, 3, vec![
                1.0, 1.0, 1.0,
                1.0, 1.0, 1.0,
                1.0, 1.0, 1.0,
            ]),
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

        assert_eq!(auto_road_travel_duration().get_duration(&world, &v2(0, 1), &v2(1, 1)), None);
    }

    #[rustfmt::skip]
    #[test]
    fn can_not_build_along_river() {
        let mut world = World::new(
            M::from_vec(3, 3, vec![
                1.0, 1.0, 1.0,
                1.0, 1.0, 1.0,
                1.0, 1.0, 1.0,
            ]),
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

        assert_eq!(auto_road_travel_duration().get_duration(&world, &v2(1, 0), &v2(1, 1)), None);
    }

    #[rustfmt::skip]
    #[test]
    fn can_cross_river_at_90_degrees() {
        let mut world = World::new(
            M::from_vec(3, 3, vec![
                1.0, 1.0, 1.0,
                1.0, 1.0, 1.0,
                1.0, 1.0, 1.0,
            ]),
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

        assert_eq!(auto_road_travel_duration().get_duration(&world, &v2(0, 1), &v2(1, 1)), 
            Some(off_road_travel_duration().max_duration()));
    }

    #[rustfmt::skip]
    #[test]
    fn can_not_build_in_sea() {
         let mut world = World::new(
            M::from_vec(3, 3, vec![
                1.0, 1.0, 0.0,
                1.0, 1.0, 0.0,
                1.0, 1.0, 0.0,
            ]),
            0.5,
        );

        world.reveal_all();

        assert_eq!(auto_road_travel_duration().get_duration(&world, &v2(2, 1), &v2(3, 1)), None);
    }

    #[rustfmt::skip]
    #[test]
    fn uses_different_travel_duration_for_existing_roads() {
        let mut world = World::new(
            M::from_vec(3, 3, vec![
                1.0, 1.0, 1.0,
                1.0, 1.0, 1.0,
                1.0, 1.0, 1.0,
            ]),
            0.5,
        );

        world.reveal_all();

        world.toggle_road(&Edge::new(v2(0, 0), v2(0, 1)));

        assert_eq!(auto_road_travel_duration().get_duration(&world, &v2(0, 0), &v2(0, 1)), Some(road_travel_duration().max_duration()));
    }

    #[rustfmt::skip]
    #[test]
    fn cannot_build_into_invisible() {
         let mut world = World::new(
            M::from_vec(3, 3, vec![
                1.0, 1.0, 1.0,
                1.0, 1.0, 1.0,
                1.0, 1.0, 1.0,
            ]),
            0.5,
        );

        world.reveal_all();
        world.mut_cell_unsafe(&v2(0, 0)).visible = false;

        assert_eq!(auto_road_travel_duration().get_duration(&world, &v2(0, 0), &v2(1, 0)), None);
    }

    #[rustfmt::skip]
    #[test]
    fn cannot_build_from_invisible() {
         let mut world = World::new(
            M::from_vec(3, 3, vec![
                1.0, 1.0, 1.0,
                1.0, 1.0, 1.0,
                1.0, 1.0, 1.0,
            ]),
            0.5,
        );

        world.reveal_all();
        world.mut_cell_unsafe(&v2(0, 0)).visible = false;

        assert_eq!(auto_road_travel_duration().get_duration(&world, &v2(1, 0), &v2(0, 0)), None);
    }

    #[rustfmt::skip]
    #[test]
    fn can_not_build_from_invisible() {
         let mut world = World::new(
            M::from_vec(3, 3, vec![
                1.0, 1.0, 1.0,
                1.0, 1.0, 1.0,
                1.0, 1.0, 1.0,
            ]),
            0.5,
        );

        world.mut_cell_unsafe(&v2(1, 0)).visible = true;

        assert_eq!(auto_road_travel_duration().get_duration(&world, &v2(0, 0), &v2(1, 0)), None);

        world.mut_cell_unsafe(&v2(0, 0)).visible = true;

        assert_eq!(auto_road_travel_duration().get_duration(&world, &v2(0, 0), &v2(1, 0)), Some(off_road_travel_duration().max_duration()));
    }

    #[test]
    fn min_duration() {
        assert_eq!(
            auto_road_travel_duration().min_duration(),
            Duration::from_millis(10)
        );
    }

    #[test]
    fn max_duration() {
        assert_eq!(
            auto_road_travel_duration().max_duration(),
            Duration::from_millis(1000)
        );
    }
}
