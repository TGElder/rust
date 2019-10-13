use crate::world::World;
use commons::edge::*;
use commons::*;

#[derive(Debug, PartialEq)]
pub enum TravelMode {
    Walk,
    Road,
    River,
    Sea,
}

#[derive(Debug, PartialEq, Clone)]
pub struct TravelModeFn {
    min_river_width: f32,
}

impl TravelModeFn {
    pub fn new(min_river_width: f32) -> TravelModeFn {
        TravelModeFn { min_river_width }
    }

    pub fn is_navigable_river_here(&self, world: &World, position: &V2<usize>) -> bool {
        if let Some(cell) = world.get_cell(position) {
            cell.river.width().max(cell.river.height()) >= self.min_river_width
        } else {
            false
        }
    }

    pub fn is_navigable_river(&self, world: &World, from: &V2<usize>, to: &V2<usize>) -> bool {
        self.is_navigable_river_here(world, from) && self.is_navigable_river_here(world, to)
    }

    pub fn travel_mode_between(
        &self,
        world: &World,
        from: &V2<usize>,
        to: &V2<usize>,
    ) -> Option<TravelMode> {
        if world.in_bounds(from) && world.in_bounds(to) {
            if world.is_sea(from) && world.is_sea(to) {
                Some(TravelMode::Sea)
            } else if world.is_road(&Edge::new(*from, *to)) {
                Some(TravelMode::Road)
            } else if self.is_navigable_river(world, from, to) {
                Some(TravelMode::River)
            } else {
                Some(TravelMode::Walk)
            }
        } else {
            None
        }
    }

    pub fn travel_mode_here(&self, world: &World, position: &V2<usize>) -> Option<TravelMode> {
        if world.in_bounds(position) {
            if world.is_sea(position) {
                Some(TravelMode::Sea)
            } else if world
                .get_cell(position)
                .map(|cell| cell.road.here())
                .unwrap_or(false)
            {
                Some(TravelMode::Road)
            } else if self.is_navigable_river_here(world, position) {
                Some(TravelMode::River)
            } else {
                Some(TravelMode::Walk)
            }
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use commons::junction::*;
    use commons::{v2, M};

    #[rustfmt::skip]
    fn world() -> World {
        let mut world = World::new(
                M::from_vec(4, 4, vec![
                    1.0, 1.0, 1.0, 0.0,
                    1.0, 1.0, 1.0, 0.0,
                    1.0, 1.0, 1.0, 0.0,
                    1.0, 1.0, 1.0, 1.0,
                ]),
                0.5,
            );

        let mut river_1 = PositionJunction::new(v2(0, 1));
        river_1.junction.horizontal.width = 0.1;
        river_1.junction.horizontal.from = true;
        let mut river_2 = PositionJunction::new(v2(1, 1));
        river_2.junction.horizontal.width = 0.2;
        river_2.junction.horizontal.from = true;
        river_2.junction.horizontal.to = true;
        let mut river_3 = PositionJunction::new(v2(2, 1));
        river_3.junction.horizontal.width = 0.3;
        river_3.junction.horizontal.to = true;
        world.add_river(river_1);
        world.add_river(river_2);
        world.add_river(river_3);
        
        world.toggle_road(&Edge::new(v2(0, 3), v2(1, 3)));
        world
    }

    fn travel_mode_fn() -> TravelModeFn {
        TravelModeFn::new(0.15)
    }

    #[test]
    fn is_navigable_river_here() {
        let world = world();
        let travel_mode_fn = travel_mode_fn();
        assert!(!travel_mode_fn.is_navigable_river_here(&world, &v2(0, 1)));
        assert!(travel_mode_fn.is_navigable_river_here(&world, &v2(1, 1)));
        assert!(!travel_mode_fn.is_navigable_river_here(&world, &v2(1, 2)));
    }

    #[test]
    fn is_navigable_river() {
        let world = world();
        let travel_mode_fn = travel_mode_fn();
        assert!(!travel_mode_fn.is_navigable_river(&world, &v2(0, 1), &v2(1, 1)));
        assert!(!travel_mode_fn.is_navigable_river(&world, &v2(1, 1), &v2(0, 1)));
        assert!(travel_mode_fn.is_navigable_river(&world, &v2(1, 1), &v2(2, 1)));
        assert!(travel_mode_fn.is_navigable_river(&world, &v2(2, 1), &v2(1, 1)));
        assert!(!travel_mode_fn.is_navigable_river(&world, &v2(0, 2), &v2(1, 2)));
        assert!(!travel_mode_fn.is_navigable_river(&world, &v2(1, 2), &v2(0, 2)));
    }

    #[test]
    fn travel_mode_in_sea() {
        let world = world();
        let travel_mode_fn = travel_mode_fn();
        assert_eq!(
            travel_mode_fn.travel_mode_between(&world, &v2(3, 0), &v2(3, 1)),
            Some(TravelMode::Sea)
        );
        assert_eq!(
            travel_mode_fn.travel_mode_between(&world, &v2(3, 1), &v2(3, 0)),
            Some(TravelMode::Sea)
        );
    }

    #[test]
    fn travel_mode_into_sea() {
        let world = world();
        let travel_mode_fn = travel_mode_fn();
        assert_eq!(
            travel_mode_fn.travel_mode_between(&world, &v2(2, 0), &v2(3, 0)),
            Some(TravelMode::Walk)
        );
        assert_eq!(
            travel_mode_fn.travel_mode_between(&world, &v2(3, 0), &v2(2, 0)),
            Some(TravelMode::Walk)
        );
    }

    #[test]
    fn travel_mode_in_river() {
        let world = world();
        let travel_mode_fn = travel_mode_fn();
        assert_eq!(
            travel_mode_fn.travel_mode_between(&world, &v2(1, 1), &v2(2, 1)),
            Some(TravelMode::River)
        );
        assert_eq!(
            travel_mode_fn.travel_mode_between(&world, &v2(2, 1), &v2(1, 1)),
            Some(TravelMode::River)
        );
    }

    #[test]
    fn travel_mode_into_river() {
        let world = world();
        let travel_mode_fn = travel_mode_fn();
        assert_eq!(
            travel_mode_fn.travel_mode_between(&world, &v2(0, 1), &v2(1, 1)),
            Some(TravelMode::Walk)
        );
        assert_eq!(
            travel_mode_fn.travel_mode_between(&world, &v2(1, 1), &v2(0, 1)),
            Some(TravelMode::Walk)
        );
    }

    #[test]
    fn travel_mode_walk() {
        let world = world();
        let travel_mode_fn = travel_mode_fn();
        assert_eq!(
            travel_mode_fn.travel_mode_between(&world, &v2(0, 0), &v2(1, 0)),
            Some(TravelMode::Walk)
        );
        assert_eq!(
            travel_mode_fn.travel_mode_between(&world, &v2(1, 0), &v2(0, 0)),
            Some(TravelMode::Walk)
        );
    }

    #[test]
    fn travel_mode_on_road() {
        let world = world();
        let travel_mode_fn = travel_mode_fn();
        assert_eq!(
            travel_mode_fn.travel_mode_between(&world, &v2(0, 3), &v2(1, 3)),
            Some(TravelMode::Road)
        );
        assert_eq!(
            travel_mode_fn.travel_mode_between(&world, &v2(1, 3), &v2(0, 3)),
            Some(TravelMode::Road)
        );
    }

    #[test]
    fn travel_mode_onto_road() {
        let world = world();
        let travel_mode_fn = travel_mode_fn();
        assert_eq!(
            travel_mode_fn.travel_mode_between(&world, &v2(0, 2), &v2(0, 3)),
            Some(TravelMode::Walk)
        );
        assert_eq!(
            travel_mode_fn.travel_mode_between(&world, &v2(0, 3), &v2(0, 2)),
            Some(TravelMode::Walk)
        );
    }

    #[test]
    fn travel_mode_out_of_bounds() {
        let world = world();
        let travel_mode_fn = travel_mode_fn();
        assert_eq!(
            travel_mode_fn.travel_mode_between(&world, &v2(3, 0), &v2(4, 0)),
            None
        );
        assert_eq!(
            travel_mode_fn.travel_mode_between(&world, &v2(4, 0), &v2(3, 0)),
            None
        );
        assert_eq!(
            travel_mode_fn.travel_mode_between(&world, &v2(4, 0), &v2(5, 0)),
            None
        );
    }

    #[test]
    fn travel_mode_bridge() {
        let mut world = world();
        let travel_mode_fn = travel_mode_fn();
        world.toggle_road(&Edge::new(v2(1, 0), v2(1, 1)));
        assert_eq!(
            travel_mode_fn.travel_mode_between(&world, &v2(1, 0), &v2(1, 1)),
            Some(TravelMode::Road)
        );
        assert_eq!(
            travel_mode_fn.travel_mode_between(&world, &v2(1, 1), &v2(1, 0)),
            Some(TravelMode::Road)
        );
    }

    #[test]
    fn travel_mode_here() {
        let world = world();
        let travel_mode_fn = travel_mode_fn();
        assert_eq!(
            travel_mode_fn.travel_mode_here(&world, &v2(3, 0)),
            Some(TravelMode::Sea)
        );
        assert_eq!(
            travel_mode_fn.travel_mode_here(&world, &v2(0, 0)),
            Some(TravelMode::Walk)
        );
        assert_eq!(
            travel_mode_fn.travel_mode_here(&world, &v2(0, 1)),
            Some(TravelMode::Walk)
        );
        assert_eq!(
            travel_mode_fn.travel_mode_here(&world, &v2(1, 1)),
            Some(TravelMode::River)
        );
        assert_eq!(
            travel_mode_fn.travel_mode_here(&world, &v2(0, 3)),
            Some(TravelMode::Road)
        );
    }

    #[test]
    fn travel_mode_here_bridge() {
        let mut world = world();
        let travel_mode_fn = travel_mode_fn();
        world.toggle_road(&Edge::new(v2(1, 0), v2(1, 1)));
        assert_eq!(
            travel_mode_fn.travel_mode_here(&world, &v2(1, 1)),
            Some(TravelMode::Road)
        );
    }
}
