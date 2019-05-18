use crate::travel_duration::*;
use crate::world::World;
use commons::V2;
use isometric::terrain::Edge;
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
        if let (Some(from_z), Some(to_z)) = (world.get_elevation(from), world.get_elevation(to)) {
            if from_z < world.sea_level() || to_z < world.sea_level() {
                None
            } else if world.is_river_corner_here(from) || world.is_river_corner_here(to) {
                None
            } else if world.is_river_here(from) && world.is_river_here(to) {
                None
            } else if world.is_road(&Edge::new(*from, *to)) {
                self.road.get_duration(world, from, to)
            } else {
                self.off_road.get_duration(world, from, to)
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
    use commons::{v2, M};
    use isometric::terrain::Node;
    use std::time::Instant;

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
        let world = World::new(
            M::from_vec(3, 3, vec![
                1.0, 1.0, 1.0,
                1.0, 1.0, 1.0,
                1.0, 1.0, 1.0,
            ]),
            vec![],
            vec![],
            0.5,
            Instant::now(),
        );

        assert_eq!(auto_road_travel_duration().get_duration(&world, &v2(0, 1), &v2(1, 1)), Some(off_road_travel_duration().max_duration()));
    }

    #[rustfmt::skip]
    #[test]
    fn can_not_build_over_river_corner() {
        let world = World::new(
            M::from_vec(3, 3, vec![
                1.0, 1.0, 1.0,
                1.0, 1.0, 1.0,
                1.0, 1.0, 1.0,
            ]),
            vec![
                Node::new(v2(1, 0), 1.0, 0.0),
                Node::new(v2(1, 1), 1.0, 1.0),
                Node::new(v2(2, 1), 0.0, 1.0),
            ],
            vec![
                Edge::new(v2(1, 0), v2(1, 1)),
                Edge::new(v2(1, 1), v2(2, 1))
            ],
            0.5,
            Instant::now(),
        );

        assert_eq!(auto_road_travel_duration().get_duration(&world, &v2(0, 1), &v2(1, 1)), None);
    }

    #[rustfmt::skip]
    #[test]
    fn can_not_build_along_river() {
        let world = World::new(
            M::from_vec(3, 3, vec![
                1.0, 1.0, 1.0,
                1.0, 1.0, 1.0,
                1.0, 1.0, 1.0,
            ]),
            vec![
                Node::new(v2(1, 0), 1.0, 0.0),
                Node::new(v2(1, 1), 1.0, 0.0),
                Node::new(v2(1, 2), 1.0, 0.0),
            ],
            vec![
                Edge::new(v2(1, 0), v2(1, 1)),
                Edge::new(v2(1, 1), v2(1, 2))
            ],
            0.5,
            Instant::now(),
        );

        assert_eq!(auto_road_travel_duration().get_duration(&world, &v2(1, 0), &v2(1, 1)), None);
    }

    #[rustfmt::skip]
    #[test]
    fn can_cross_river_at_90_degrees() {
        let world = World::new(
            M::from_vec(3, 3, vec![
                1.0, 1.0, 1.0,
                1.0, 1.0, 1.0,
                1.0, 1.0, 1.0,
            ]),
            vec![
                Node::new(v2(1, 0), 1.0, 0.0),
                Node::new(v2(1, 1), 1.0, 0.0),
                Node::new(v2(1, 2), 1.0, 0.0),
            ],
            vec![
                Edge::new(v2(1, 0), v2(1, 1)),
                Edge::new(v2(1, 1), v2(1, 2))
            ],
            0.5,
            Instant::now(),
        );

        assert_eq!(auto_road_travel_duration().get_duration(&world, &v2(0, 1), &v2(1, 1)), 
            Some(off_road_travel_duration().max_duration()));
    }

    #[rustfmt::skip]
    #[test]
    fn can_not_build_into_sea() {
         let world = World::new(
            M::from_vec(3, 3, vec![
                1.0, 1.0, 0.0,
                1.0, 1.0, 0.0,
                1.0, 1.0, 0.0,
            ]),
            vec![],
            vec![],
            0.5,
            Instant::now(),
        );

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
            vec![],
            vec![],
            0.5,
            Instant::now(),
        );

        world.toggle_road(&Edge::new(v2(0, 0), v2(0, 1)));

        assert_eq!(auto_road_travel_duration().get_duration(&world, &v2(0, 0), &v2(0, 1)), Some(road_travel_duration().max_duration()));
    }
}
