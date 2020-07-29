use crate::world::World;
use commons::edge::*;
use commons::*;
use std::collections::HashSet;

#[derive(Debug, PartialEq)]
pub enum TravelMode {
    Walk,
    Road,
    Stream,
    River,
    Sea,
}

#[derive(Debug, Eq, Hash, PartialEq)]
pub enum TravelModeClass {
    Land,
    Water,
}

impl TravelMode {
    pub fn class(&self) -> TravelModeClass {
        match self {
            TravelMode::Walk => TravelModeClass::Land,
            TravelMode::Road => TravelModeClass::Land,
            TravelMode::Stream => TravelModeClass::Land,
            TravelMode::River => TravelModeClass::Water,
            TravelMode::Sea => TravelModeClass::Water,
        }
    }
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
        world.is_river(&Edge::new(*from, *to))
            && self.is_navigable_river_here(world, from)
            && self.is_navigable_river_here(world, to)
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
            } else if world.is_road(&Edge::new(*from, *to))
                || world.road_planned(&Edge::new(*from, *to)).is_some()
            {
                Some(TravelMode::Road)
            } else if self.is_navigable_river(world, from, to) {
                Some(TravelMode::River)
            } else if world.is_river(&Edge::new(*from, *to)) {
                Some(TravelMode::Stream)
            } else {
                Some(TravelMode::Walk)
            }
        } else {
            None
        }
    }

    pub fn travel_modes_here(&self, world: &World, position: &V2<usize>) -> Vec<TravelMode> {
        let mut out = vec![];
        if let Some(cell) = world.get_cell(position) {
            if world.is_sea(position) {
                out.push(TravelMode::Sea);
            } else {
                if cell.road.here() || cell.planned_road.is_some() {
                    out.push(TravelMode::Road);
                }
                if self.is_navigable_river_here(world, position) {
                    out.push(TravelMode::River);
                } else if cell.river.here() {
                    out.push(TravelMode::Stream);
                }
                if out.is_empty() {
                    out.push(TravelMode::Walk);
                }
            }
        }
        out
    }

    pub fn travel_mode_change(&self, world: &World, from: &V2<usize>, to: &V2<usize>) -> bool {
        let from_classes = self.travel_mode_classes_here(world, from);
        let to_classes = self.travel_mode_classes_here(world, to);
        !from_classes.intersection(&to_classes).any(|_| true)
    }

    #[allow(dead_code)] // TODO
    pub fn check_for_port(
        &self,
        world: &World,
        from: &V2<usize>,
        to: &V2<usize>,
    ) -> Option<V2<usize>> {
        let from_water = self.is_water_here(world, from);
        let to_water = self.is_water_here(world, to);
        if from_water && !to_water {
            Some(*to)
        } else if !from_water && to_water {
            Some(*from)
        } else {
            None
        }
    }

    fn is_water_here(&self, world: &World, position: &V2<usize>) -> bool {
        self.travel_mode_classes_here(world, position)
            .contains(&TravelModeClass::Water)
    }

    fn travel_mode_classes_here(
        &self,
        world: &World,
        position: &V2<usize>,
    ) -> HashSet<TravelModeClass> {
        self.travel_modes_here(world, position)
            .into_iter()
            .map(|mode| mode.class())
            .collect()
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
        
        world.set_road(&Edge::new(v2(0, 3), v2(1, 3)), true);
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
    fn travel_mode_in_stream() {
        let world = world();
        let travel_mode_fn = TravelModeFn::new(0.5);
        assert_eq!(
            travel_mode_fn.travel_mode_between(&world, &v2(1, 1), &v2(2, 1)),
            Some(TravelMode::Stream)
        );
        assert_eq!(
            travel_mode_fn.travel_mode_between(&world, &v2(2, 1), &v2(1, 1)),
            Some(TravelMode::Stream)
        );
    }

    #[test]
    fn travel_mode_into_stream() {
        let world = world();
        let travel_mode_fn = TravelModeFn::new(0.5);
        assert_eq!(
            travel_mode_fn.travel_mode_between(&world, &v2(0, 0), &v2(0, 1)),
            Some(TravelMode::Walk)
        );
        assert_eq!(
            travel_mode_fn.travel_mode_between(&world, &v2(0, 1), &v2(0, 0)),
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
            travel_mode_fn.travel_mode_between(&world, &v2(0, 0), &v2(0, 1)),
            Some(TravelMode::Walk)
        );
        assert_eq!(
            travel_mode_fn.travel_mode_between(&world, &v2(0, 1), &v2(0, 0)),
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
    fn travel_mode_on_planned_road() {
        let mut world = world();
        let travel_mode_fn = travel_mode_fn();
        let edge = Edge::new(v2(0, 3), v2(1, 3));
        world.set_road(&edge, false);
        world.plan_road(&edge, true, 0);
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
    fn travel_mode_onto_planned_road() {
        let mut world = world();
        let travel_mode_fn = travel_mode_fn();
        let edge = Edge::new(v2(0, 3), v2(1, 3));
        world.set_road(&edge, false);
        world.plan_road(&edge, true, 0);
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
        world.set_road(&Edge::new(v2(1, 0), v2(1, 1)), true);
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
    fn travel_mode_planned_bridge() {
        let mut world = world();
        let travel_mode_fn = travel_mode_fn();
        world.plan_road(&Edge::new(v2(1, 0), v2(1, 1)), true, 0);
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
            travel_mode_fn.travel_modes_here(&world, &v2(3, 0)),
            vec![TravelMode::Sea]
        );
        assert_eq!(
            travel_mode_fn.travel_modes_here(&world, &v2(0, 0)),
            vec![TravelMode::Walk]
        );
        assert_eq!(
            travel_mode_fn.travel_modes_here(&world, &v2(0, 1)),
            vec![TravelMode::Stream]
        );
        assert_eq!(
            travel_mode_fn.travel_modes_here(&world, &v2(1, 1)),
            vec![TravelMode::River]
        );
        assert_eq!(
            travel_mode_fn.travel_modes_here(&world, &v2(0, 3)),
            vec![TravelMode::Road]
        );
    }

    #[test]
    fn travel_mode_here_planned_road() {
        let mut world = world();
        let travel_mode_fn = travel_mode_fn();
        let edge = Edge::new(v2(0, 3), v2(1, 3));
        world.set_road(&edge, false);
        world.plan_road(&edge, true, 0);
        assert_eq!(
            travel_mode_fn.travel_modes_here(&world, &v2(0, 3)),
            vec![TravelMode::Road]
        );
    }

    #[test]
    fn travel_mode_here_bridge() {
        let mut world = world();
        let travel_mode_fn = travel_mode_fn();
        world.set_road(&Edge::new(v2(1, 0), v2(1, 1)), true);
        assert!(same_elements(
            &travel_mode_fn.travel_modes_here(&world, &v2(1, 1)),
            &[TravelMode::Road, TravelMode::River]
        ))
    }

    #[test]
    fn travel_mode_here_planned_bridge() {
        let mut world = world();
        let travel_mode_fn = travel_mode_fn();
        world.plan_road(&Edge::new(v2(1, 0), v2(1, 1)), true, 0);
        assert!(same_elements(
            &travel_mode_fn.travel_modes_here(&world, &v2(1, 1)),
            &[TravelMode::Road, TravelMode::River]
        ))
    }

    #[test]
    #[rustfmt::skip]
    fn inside_of_river_u_bend_should_not_cound_as_river() {
        let mut world = World::new(
            M::from_vec(
                4,
                4,
                vec![
                    1.0, 1.0, 1.0, 0.0,
                    1.0, 1.0, 1.0, 0.0,
                    1.0, 1.0, 1.0, 0.0,
                    1.0, 1.0, 1.0, 1.0,
                ],
            ),
            0.5,
        );

        let mut river_1 = PositionJunction::new(v2(0, 0));
        river_1.junction.horizontal.width = 0.2;
        river_1.junction.horizontal.from = true;
        let mut river_2 = PositionJunction::new(v2(1, 0));
        river_2.junction.horizontal.width = 0.2;
        river_2.junction.horizontal.to = true;
        river_2.junction.vertical.from = true;
        let mut river_3 = PositionJunction::new(v2(1, 1));
        river_3.junction.horizontal.width = 0.2;
        river_2.junction.horizontal.to = true;
        river_2.junction.vertical.to = true;
        let mut river_4 = PositionJunction::new(v2(0, 1));
        river_4.junction.horizontal.width = 0.2;
        river_2.junction.horizontal.from = true;
        world.add_river(river_1);
        world.add_river(river_2);
        world.add_river(river_3);
        world.add_river(river_4);

        assert_eq!(
            travel_mode_fn().travel_mode_between(&world, &v2(0, 0), &v2(0, 1)),
            Some(TravelMode::Walk)
        );
    }

    #[test]
    #[rustfmt::skip]
    fn inside_of_road_u_bend_should_not_count_as_road() {
        let mut world = World::new(
            M::from_vec(
                4,
                4,
                vec![
                    1.0, 1.0, 1.0, 0.0,
                    1.0, 1.0, 1.0, 0.0,
                    1.0, 1.0, 1.0, 0.0,
                    1.0, 1.0, 1.0, 1.0,
                ],
            ),
            0.5,
        );

        world.set_road(&Edge::new(v2(0, 0), v2(1, 0)), true);
        world.set_road(&Edge::new(v2(1, 0), v2(1, 1)), true);
        world.set_road(&Edge::new(v2(1, 1), v2(0, 1)), true);

        assert_eq!(
            travel_mode_fn().travel_mode_between(&world, &v2(0, 0), &v2(0, 1)),
            Some(TravelMode::Walk)
        );
    }

    fn world_with_bridge() -> World {
        let mut world = world();
        world.set_road(&Edge::new(v2(1, 0), v2(2, 0)), true);
        world.set_road(&Edge::new(v2(1, 0), v2(1, 1)), true);
        world.set_road(&Edge::new(v2(1, 1), v2(1, 2)), true);
        world.set_road(&Edge::new(v2(1, 2), v2(1, 3)), true);
        world
    }

    fn test_travel_mode_change_and_port(
        world: World,
        from: V2<usize>,
        to: V2<usize>,
        expected: bool,
        port: Option<V2<usize>>,
    ) {
        let travel_mode_fn = travel_mode_fn();
        assert_eq!(
            travel_mode_fn.travel_mode_change(&world, &from, &to),
            expected
        );
        assert_eq!(
            travel_mode_fn.travel_mode_change(&world, &to, &from),
            expected
        );
        assert_eq!(travel_mode_fn.check_for_port(&world, &to, &from), port);
    }

    #[test]
    fn travel_mode_change_and_port_walk_to_walk() {
        test_travel_mode_change_and_port(world_with_bridge(), v2(2, 2), v2(2, 3), false, None);
    }

    #[test]
    fn travel_mode_change_and_port_walk_to_road() {
        test_travel_mode_change_and_port(world_with_bridge(), v2(2, 2), v2(1, 2), false, None);
    }

    #[test]
    fn travel_mode_change_and_port_walk_to_stream() {
        test_travel_mode_change_and_port(world_with_bridge(), v2(0, 0), v2(0, 1), false, None);
    }

    #[test]
    fn travel_mode_change_and_port_walk_to_river() {
        test_travel_mode_change_and_port(
            world_with_bridge(),
            v2(2, 2),
            v2(2, 1),
            true,
            Some(v2(2, 2)),
        );
    }

    #[test]
    fn travel_mode_change_and_port_walk_to_sea() {
        test_travel_mode_change_and_port(
            world_with_bridge(),
            v2(2, 2),
            v2(3, 2),
            true,
            Some(v2(2, 2)),
        );
    }

    #[test]
    fn travel_mode_change_and_port_walk_to_bridge() {
        // Not possible
    }

    #[test]
    fn travel_mode_change_and_port_road_to_road() {
        test_travel_mode_change_and_port(world_with_bridge(), v2(1, 2), v2(1, 3), false, None);
    }

    #[test]
    fn travel_mode_change_and_port_road_to_stream() {
        let mut world = world_with_bridge();
        world.set_road(&Edge::new(v2(0, 0), v2(1, 0)), true);
        test_travel_mode_change_and_port(world, v2(0, 0), v2(0, 1), false, None);
    }

    #[test]
    fn travel_mode_change_and_port_road_to_river() {
        test_travel_mode_change_and_port(
            world_with_bridge(),
            v2(2, 0),
            v2(2, 1),
            true,
            Some(v2(2, 0)),
        );
    }

    #[test]
    fn travel_mode_change_and_port_road_to_sea() {
        test_travel_mode_change_and_port(
            world_with_bridge(),
            v2(2, 0),
            v2(3, 0),
            true,
            Some(v2(2, 0)),
        );
    }

    #[test]
    fn travel_mode_change_and_port_road_to_bridge() {
        test_travel_mode_change_and_port(
            world_with_bridge(),
            v2(1, 0),
            v2(1, 1),
            false,
            Some(v2(1, 0)),
        );
    }

    #[test]
    fn travel_mode_change_and_port_stream_to_river() {
        test_travel_mode_change_and_port(world(), v2(0, 1), v2(1, 1), true, Some(v2(0, 1)));
    }

    #[test]
    fn travel_mode_change_and_port_stream_to_sea() {
        let mut world = world();
        world.mut_cell_unsafe(&v2(0, 0)).elevation = 0.0;
        test_travel_mode_change_and_port(world, v2(0, 1), v2(0, 0), true, Some(v2(0, 1)));
    }

    #[test]
    fn travel_mode_change_and_port_stream_to_bridge() {
        test_travel_mode_change_and_port(
            world_with_bridge(),
            v2(0, 1),
            v2(1, 1),
            false,
            Some(v2(0, 1)),
        );
    }

    #[test]
    fn travel_mode_change_and_port_river_to_river() {
        test_travel_mode_change_and_port(world(), v2(1, 1), v2(2, 1), false, None);
    }

    #[test]
    fn travel_mode_change_and_port_river_to_sea() {
        test_travel_mode_change_and_port(world_with_bridge(), v2(2, 1), v2(3, 1), false, None);
    }

    #[test]
    fn travel_mode_change_and_port_river_to_bridge() {
        test_travel_mode_change_and_port(world_with_bridge(), v2(2, 1), v2(1, 1), false, None);
    }

    #[test]
    fn travel_mode_change_and_port_sea_to_sea() {
        test_travel_mode_change_and_port(world_with_bridge(), v2(3, 0), v2(3, 1), false, None);
    }

    #[test]
    fn travel_mode_change_and_port_sea_to_bridge() {
        let mut world = world();
        world.set_road(&Edge::new(v2(2, 0), v2(2, 1)), true);
        world.set_road(&Edge::new(v2(2, 1), v2(1, 2)), true);
        test_travel_mode_change_and_port(world, v2(3, 1), v2(2, 1), false, None);
    }

    #[test]
    fn travel_mode_change_and_port_bridge_to_bridge() {
        let mut world = world_with_bridge();
        world.set_road(&Edge::new(v2(2, 0), v2(2, 1)), true);
        world.set_road(&Edge::new(v2(2, 1), v2(1, 2)), true);
        test_travel_mode_change_and_port(world, v2(1, 1), v2(2, 1), false, None);
    }
}
