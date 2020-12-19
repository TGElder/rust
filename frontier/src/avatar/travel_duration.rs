use super::*;

use crate::travel_duration::*;
use crate::world::{World, WorldCell};
use commons::grid::Grid;
use commons::scale::*;
use commons::*;
use serde::{Deserialize, Serialize};
use std::default::Default;
use std::time::Duration;

#[derive(PartialEq, Debug, Serialize, Deserialize)]
pub struct AvatarTravelParams {
    pub max_walk_gradient: f32,
    pub walk_1_cell_duration_millis_range: (f32, f32),
    pub stream_1_cell_duration_millis_range: (f32, f32),
    pub min_navigable_river_width: f32,
    pub max_navigable_river_gradient: f32,
    pub river_1_cell_duration_millis: f32,
    pub road_1_cell_duration_millis: u64,
    pub sea_1_cell_duration_millis: u64,
    pub travel_mode_change_penalty_millis: u64,
}

impl Default for AvatarTravelParams {
    fn default() -> AvatarTravelParams {
        AvatarTravelParams {
            max_walk_gradient: 0.5,
            walk_1_cell_duration_millis_range: (2_400_000.0, 4_800_000.0),
            stream_1_cell_duration_millis_range: (4_800_000.0, 9_600_000.0),
            min_navigable_river_width: 0.1,
            max_navigable_river_gradient: 0.1,
            river_1_cell_duration_millis: 900_000.0,
            road_1_cell_duration_millis: 1_200_000,
            sea_1_cell_duration_millis: 900_000,
            travel_mode_change_penalty_millis: 3_600_000,
        }
    }
}

pub struct AvatarTravelDuration {
    travel_mode_fn: AvatarTravelModeFn,
    walk: Box<dyn TravelDuration>,
    road: Box<dyn TravelDuration>,
    planned_road: Box<dyn TravelDuration>,
    stream: Box<dyn TravelDuration>,
    river: Box<dyn TravelDuration>,
    sea: Box<dyn TravelDuration>,
    travel_mode_change_penalty_millis: u64,
}

impl AvatarTravelDuration {
    pub fn with_planned_roads_as_roads(p: &AvatarTravelParams) -> AvatarTravelDuration {
        AvatarTravelDuration {
            travel_mode_fn: AvatarTravelModeFn::new(p.min_navigable_river_width),
            walk: Self::walk(p),
            road: Self::road(p),
            planned_road: Self::road(p),
            stream: Self::stream(p),
            river: Self::river(p),
            sea: Self::sea(p),
            travel_mode_change_penalty_millis: p.travel_mode_change_penalty_millis,
        }
    }

    pub fn with_planned_roads_ignored(p: &AvatarTravelParams) -> AvatarTravelDuration {
        AvatarTravelDuration {
            travel_mode_fn: AvatarTravelModeFn::new(p.min_navigable_river_width),
            walk: Self::walk(p),
            road: Self::road(p),
            planned_road: Self::walk(p),
            stream: Self::stream(p),
            river: Self::river(p),
            sea: Self::sea(p),
            travel_mode_change_penalty_millis: p.travel_mode_change_penalty_millis,
        }
    }

    fn walk(p: &AvatarTravelParams) -> Box<dyn TravelDuration> {
        NoRiverCornersTravelDuration::boxed(GradientTravelDuration::boxed(
            Scale::new(
                (0.0, p.max_walk_gradient),
                p.walk_1_cell_duration_millis_range,
            ),
            true,
        ))
    }

    fn stream(p: &AvatarTravelParams) -> Box<dyn TravelDuration> {
        GradientTravelDuration::boxed(
            Scale::new(
                (0.0, p.max_walk_gradient),
                p.stream_1_cell_duration_millis_range,
            ),
            true,
        )
    }

    fn river(p: &AvatarTravelParams) -> Box<dyn TravelDuration> {
        GradientTravelDuration::boxed(
            Scale::new(
                (
                    -p.max_navigable_river_gradient,
                    p.max_navigable_river_gradient,
                ),
                (
                    p.river_1_cell_duration_millis,
                    p.river_1_cell_duration_millis,
                ),
            ),
            false,
        )
    }

    fn road(p: &AvatarTravelParams) -> Box<dyn TravelDuration> {
        ConstantTravelDuration::boxed(Duration::from_millis(p.road_1_cell_duration_millis))
    }

    fn sea(p: &AvatarTravelParams) -> Box<dyn TravelDuration> {
        ConstantTravelDuration::boxed(Duration::from_millis(p.sea_1_cell_duration_millis))
    }
}

impl AvatarTravelDuration {
    fn get_duration_fn(
        &self,
        world: &World,
        from: &V2<usize>,
        to: &V2<usize>,
    ) -> Option<&dyn TravelDuration> {
        self.travel_mode_fn
            .travel_mode_between(world, from, to)
            .map(|travel_mode| match travel_mode {
                TravelMode::Walk => self.walk.as_ref(),
                TravelMode::Road => self.road.as_ref(),
                TravelMode::PlannedRoad => self.planned_road.as_ref(),
                TravelMode::Stream => self.stream.as_ref(),
                TravelMode::River => self.river.as_ref(),
                TravelMode::Sea => self.sea.as_ref(),
            })
    }

    fn travel_mode_change_penalty(
        &self,
        world: &World,
        from: &V2<usize>,
        to: &V2<usize>,
    ) -> Duration {
        if self.travel_mode_fn.travel_mode_change(world, from, to) {
            Duration::from_millis(self.travel_mode_change_penalty_millis)
        } else {
            Duration::from_millis(0)
        }
    }
}

impl TravelDuration for AvatarTravelDuration {
    fn get_duration(&self, world: &World, from: &V2<usize>, to: &V2<usize>) -> Option<Duration> {
        match world.get_cell(from) {
            Some(WorldCell { visible: true, .. }) => (),
            _ => return None,
        };
        match world.get_cell(to) {
            Some(WorldCell { visible: true, .. }) => (),
            _ => return None,
        };
        self.get_duration_fn(world, from, to)
            .and_then(|duration_fn| duration_fn.get_duration(world, from, to))
            .map(|duration| duration + self.travel_mode_change_penalty(world, from, to))
    }

    fn min_duration(&self) -> Duration {
        self.walk
            .min_duration()
            .min(self.road.min_duration())
            .min(self.stream.max_duration())
            .min(self.river.min_duration())
    }

    fn max_duration(&self) -> Duration {
        self.walk
            .max_duration()
            .max(self.road.max_duration())
            .max(self.stream.max_duration())
            .max(self.river.max_duration())
            + Duration::from_millis(self.travel_mode_change_penalty_millis)
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    fn avatar_travel_duration() -> AvatarTravelDuration {
        AvatarTravelDuration {
            travel_mode_fn: AvatarTravelModeFn::new(0.5),
            walk: test_travel_duration(),
            road: test_travel_duration(),
            planned_road: test_travel_duration(),
            stream: test_travel_duration(),
            river: test_travel_duration(),
            sea: test_travel_duration(),
            travel_mode_change_penalty_millis: 100,
        }
    }

    fn test_travel_duration() -> Box<dyn TravelDuration> {
        ConstantTravelDuration::boxed(Duration::from_millis(10))
    }

    #[rustfmt::skip]
    #[test]
    fn cannot_travel_into_invisible() {
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

        assert_eq!(avatar_travel_duration().get_duration(&world, &v2(0, 0), &v2(1, 0)), None);
    }

    #[rustfmt::skip]
    #[test]
    fn cannot_travel_from_invisible() {
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

        assert_eq!(avatar_travel_duration().get_duration(&world, &v2(1, 0), &v2(0, 0)), None);
    }
}
