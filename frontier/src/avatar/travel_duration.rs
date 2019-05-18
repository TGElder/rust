use crate::travel_duration::*;
use crate::world::World;
use commons::V2;
use isometric::terrain::Edge;
use std::time::Duration;

pub struct AvatarTravelDuration {
    walk: Box<TravelDuration>,
    road: Box<TravelDuration>,
    river: Box<TravelDuration>,
}

impl AvatarTravelDuration {
    pub fn new(
        walk: Box<TravelDuration>,
        road: Box<TravelDuration>,
        river: Box<TravelDuration>,
    ) -> AvatarTravelDuration {
        AvatarTravelDuration { walk, road, river }
    }

    pub fn boxed(
        walk: Box<TravelDuration>,
        road: Box<TravelDuration>,
        river: Box<TravelDuration>,
    ) -> Box<TravelDuration> {
        Box::new(AvatarTravelDuration::new(walk, road, river))
    }
}

impl TravelDuration for AvatarTravelDuration {
    fn get_duration(&self, world: &World, from: &V2<usize>, to: &V2<usize>) -> Option<Duration> {
        if let (Some(from_z), Some(to_z)) = (world.get_elevation(from), world.get_elevation(to)) {
            if from_z < world.sea_level() || to_z < world.sea_level() {
                None
            } else if world.is_road(&Edge::new(*from, *to)) {
                self.road.get_duration(world, from, to)
            } else if world.is_river(&Edge::new(*from, *to)) {
                self.river.get_duration(world, from, to)
            } else {
                self.walk.get_duration(world, from, to)
            }
        } else {
            None
        }
    }

    fn max_duration(&self) -> Duration {
        self.walk
            .max_duration()
            .max(self.road.max_duration())
            .max(self.river.max_duration())
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use commons::{v2, M};
    use isometric::terrain::Node;
    use std::time::Instant;

    fn walk_travel_duration() -> Box<TravelDuration> {
        ConstantTravelDuration::boxed(Duration::from_millis(1))
    }

    fn road_travel_duration() -> Box<TravelDuration> {
        ConstantTravelDuration::boxed(Duration::from_millis(2))
    }

    fn river_travel_duration() -> Box<TravelDuration> {
        ConstantTravelDuration::boxed(Duration::from_millis(3))
    }

    fn avatar_travel_duration() -> AvatarTravelDuration {
        AvatarTravelDuration::new(
            walk_travel_duration(),
            road_travel_duration(),
            river_travel_duration(),
        )
    }

    #[rustfmt::skip]
    #[test]
    fn can_not_walk_into_sea() {
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

        assert_eq!(avatar_travel_duration().get_duration(&world, &v2(1, 1), &v2(2, 1)), None);
    }

    #[rustfmt::skip]
    #[test]
    fn defaults_to_walk_travel_duration() {
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

        assert_eq!(avatar_travel_duration().get_duration(&world, &v2(0, 1), &v2(1, 1)), Some(walk_travel_duration().max_duration()));
    }

    #[rustfmt::skip]
    #[test]
    fn uses_river_travel_duration_along_river() {
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

        assert_eq!(avatar_travel_duration().get_duration(&world, &v2(1, 0), &v2(1, 1)), Some(river_travel_duration().max_duration()));
    }

    #[rustfmt::skip]
    #[test]
    fn uses_road_travel_direction_along_road() {
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

        assert_eq!(avatar_travel_duration().get_duration(&world, &v2(0, 0), &v2(0, 1)), Some(road_travel_duration().max_duration()));
    }
}
