use super::*;

use crate::world::World;
use commons::edge::*;
use commons::grid::Grid;
use commons::*;

#[derive(Debug, PartialEq, Clone)]
pub struct AvatarTravelModeFn {
    min_river_width: f32,
    include_planned_roads: bool,
}

impl TravelModeFn for AvatarTravelModeFn {
    fn travel_mode_between(
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
            } else if self.include_planned_roads
                && world.road_planned(&Edge::new(*from, *to)).is_some()
            {
                Some(TravelMode::PlannedRoad)
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

    fn travel_modes_here(&self, world: &World, position: &V2<usize>) -> Vec<TravelMode> {
        let mut out = vec![];
        if let Some(cell) = world.get_cell(position) {
            if cell.road.here() {
                out.push(TravelMode::Road);
            } else if self.include_planned_roads && cell.planned_road.here() {
                out.push(TravelMode::PlannedRoad);
            }
            if world.is_sea(position) {
                out.push(TravelMode::Sea);
            } else if self.is_navigable_river_here(world, position) {
                out.push(TravelMode::River);
            } else if cell.river.here() {
                out.push(TravelMode::Stream);
            }
            if out.is_empty() {
                out.push(TravelMode::Walk);
            }
        }
        out
    }
}

impl AvatarTravelModeFn {
    pub fn new(min_river_width: f32, include_planned_roads: bool) -> AvatarTravelModeFn {
        AvatarTravelModeFn {
            min_river_width,
            include_planned_roads,
        }
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

    fn travel_mode_fn() -> AvatarTravelModeFn {
        AvatarTravelModeFn::new(0.15, true)
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
        let travel_mode_fn = AvatarTravelModeFn::new(0.5, true);
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
        let travel_mode_fn = AvatarTravelModeFn::new(0.5, true);
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
    fn travel_mode_on_planned_road_include_planned_roads() {
        let mut world = world();
        let travel_mode_fn = AvatarTravelModeFn::new(0.15, true);
        let edge = Edge::new(v2(0, 3), v2(1, 3));
        world.set_road(&edge, false);
        world.plan_road(&edge, Some(0));
        assert_eq!(
            travel_mode_fn.travel_mode_between(&world, &v2(0, 3), &v2(1, 3)),
            Some(TravelMode::PlannedRoad)
        );
        assert_eq!(
            travel_mode_fn.travel_mode_between(&world, &v2(1, 3), &v2(0, 3)),
            Some(TravelMode::PlannedRoad)
        );
    }

    #[test]
    fn travel_mode_on_planned_road_ignore_planned_roads() {
        let mut world = world();
        let travel_mode_fn = AvatarTravelModeFn::new(0.15, false);
        let edge = Edge::new(v2(0, 3), v2(1, 3));
        world.set_road(&edge, false);
        world.plan_road(&edge, Some(0));
        assert_eq!(
            travel_mode_fn.travel_mode_between(&world, &v2(0, 3), &v2(1, 3)),
            Some(TravelMode::Walk)
        );
        assert_eq!(
            travel_mode_fn.travel_mode_between(&world, &v2(1, 3), &v2(0, 3)),
            Some(TravelMode::Walk)
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
        world.plan_road(&edge, Some(0));
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
        world.plan_road(&Edge::new(v2(1, 0), v2(1, 1)), Some(0));
        assert_eq!(
            travel_mode_fn.travel_mode_between(&world, &v2(1, 0), &v2(1, 1)),
            Some(TravelMode::PlannedRoad)
        );
        assert_eq!(
            travel_mode_fn.travel_mode_between(&world, &v2(1, 1), &v2(1, 0)),
            Some(TravelMode::PlannedRoad)
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
    fn travel_mode_here_planned_road_include_planned_roads() {
        let mut world = world();
        let travel_mode_fn = AvatarTravelModeFn::new(0.15, true);
        let edge = Edge::new(v2(0, 3), v2(1, 3));
        world.set_road(&edge, false);
        world.plan_road(&edge, Some(0));
        assert_eq!(
            travel_mode_fn.travel_modes_here(&world, &v2(0, 3)),
            vec![TravelMode::PlannedRoad]
        );
    }

    #[test]
    fn travel_mode_here_planned_road_ignore_planned_roads() {
        let mut world = world();
        let travel_mode_fn = AvatarTravelModeFn::new(0.15, false);
        let edge = Edge::new(v2(0, 3), v2(1, 3));
        world.set_road(&edge, false);
        world.plan_road(&edge, Some(0));
        assert_eq!(
            travel_mode_fn.travel_modes_here(&world, &v2(0, 3)),
            vec![TravelMode::Walk]
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
        world.plan_road(&Edge::new(v2(1, 0), v2(1, 1)), Some(0));
        assert!(same_elements(
            &travel_mode_fn.travel_modes_here(&world, &v2(1, 1)),
            &[TravelMode::PlannedRoad, TravelMode::River]
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
}
