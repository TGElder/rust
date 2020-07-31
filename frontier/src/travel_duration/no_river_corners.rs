use super::*;

use commons::grid::*;

pub struct NoRiverCornersTravelDuration<T>
where
    T: TravelDuration,
{
    base: Box<T>,
}

impl<T> NoRiverCornersTravelDuration<T>
where
    T: TravelDuration,
{
    pub fn new(base: Box<T>) -> NoRiverCornersTravelDuration<T> {
        NoRiverCornersTravelDuration { base }
    }

    pub fn boxed(base: Box<T>) -> Box<NoRiverCornersTravelDuration<T>> {
        Box::new(Self::new(base))
    }
}

fn river_corner(world: &World, position: &V2<usize>) -> bool {
    let cell = unwrap_or!(world.get_cell(position), return true);
    cell.river.corner()
}

impl<T> TravelDuration for NoRiverCornersTravelDuration<T>
where
    T: TravelDuration,
{
    fn get_duration(&self, world: &World, from: &V2<usize>, to: &V2<usize>) -> Option<Duration> {
        if river_corner(world, from) || river_corner(world, to) {
            return None;
        }
        self.base.get_duration(world, from, to)
    }

    fn min_duration(&self) -> Duration {
        self.base.min_duration()
    }

    fn max_duration(&self) -> Duration {
        self.base.max_duration()
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::world::World;
    use commons::junction::*;
    use commons::{v2, M};

    fn travel_duration() -> NoRiverCornersTravelDuration<ConstantTravelDuration> {
        NoRiverCornersTravelDuration::new(ConstantTravelDuration::boxed(Duration::from_millis(0)))
    }

    #[rustfmt::skip]
    fn world() -> World {
        let mut world = World::new(
            M::zeros(3, 3),
            0.0,
        );

        let mut river_1 = PositionJunction::new(v2(1, 0));
        river_1.junction.horizontal.width = 1.0;
        river_1.junction.vertical.from = true;
        let mut river_2 = PositionJunction::new(v2(1, 1));
        river_2.junction.horizontal.width = 1.0;
        river_2.junction.vertical.width = 1.0;
        river_2.junction.vertical.to = true;
        river_2.junction.horizontal.from = true;
        let mut river_3 = PositionJunction::new(v2(2, 1));
        river_3.junction.horizontal.width = 1.0;
        river_3.junction.horizontal.to = true;
        world.add_river(river_1);
        world.add_river(river_2);
        world.add_river(river_3);

        world
    }

    #[test]
    fn cannot_walk_from_corner() {
        assert_eq!(
            travel_duration().get_duration(&world(), &v2(1, 1), &v2(1, 2)),
            None
        );
    }

    #[test]
    fn cannot_walk_into_corner() {
        assert_eq!(
            travel_duration().get_duration(&world(), &v2(1, 2), &v2(1, 1)),
            None
        );
    }

    #[test]
    fn can_walk_elsewhere() {
        assert!(travel_duration()
            .get_duration(&world(), &v2(0, 0), &v2(1, 0))
            .is_some());
    }
}
