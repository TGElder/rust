use crate::travel_duration::*;
use crate::world::World;
use commons::edge::*;
use commons::*;
use isometric::cell_traits::*;
use std::time::Duration;

pub struct AutoRoadTravelDuration {
    off_road: Box<TravelDuration>,
    road: Box<TravelDuration>,
}

impl AutoRoadTravelDuration {
    pub fn new(off_road: Box<TravelDuration>, road: Box<TravelDuration>) -> AutoRoadTravelDuration {
        AutoRoadTravelDuration { off_road, road }
    }
}

impl TravelDuration for AutoRoadTravelDuration {
    fn get_duration(&self, world: &World, from: &V2<usize>, to: &V2<usize>) -> Option<Duration> {
        if !world
            .get_cell(from)
            .map(|cell| cell.is_visible())
            .unwrap_or(false)
        {
            return None;
        }
        if let (Some(from), Some(to)) = (world.get_cell(from), world.get_cell(to)) {
            if from.elevation() < world.sea_level() || to.elevation() < world.sea_level() {
                None
            } else if from.river.corner() || to.river.corner() {
                None
            } else if from.river.here() && to.river.here() {
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

    fn max_duration(&self) -> Duration {
        Duration::from_millis(1000)
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use commons::junction::*;
    use commons::{v2, M};

    fn road_travel_duration() -> Box<TravelDuration> {
        ConstantTravelDuration::boxed(Duration::from_millis(10))
    }

    fn off_road_travel_duration() -> Box<TravelDuration> {
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
    fn can_not_build_into_sea() {
         let mut world = World::new(
            M::from_vec(3, 3, vec![
                1.0, 1.0, 0.0,
                1.0, 1.0, 0.0,
                1.0, 1.0, 0.0,
            ]),
            0.5,
        );

        world.reveal_all();

        assert_eq!(auto_road_travel_duration().get_duration(&world, &v2(1, 1), &v2(2, 1)), None);
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
    fn can_build_into_invisible() {
         let mut world = World::new(
            M::from_vec(3, 3, vec![
                1.0, 1.0, 1.0,
                1.0, 1.0, 1.0,
                1.0, 1.0, 1.0,
            ]),
            0.5,
        );

        world.mut_cell_unsafe(&v2(0, 0)).visible = true;

        assert_eq!(auto_road_travel_duration().get_duration(&world, &v2(0, 0), &v2(1, 0)), Some(off_road_travel_duration().max_duration()));
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
}
