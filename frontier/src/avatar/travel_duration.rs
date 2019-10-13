use super::travel_mode::*;
use crate::travel_duration::*;
use crate::world::World;
use commons::scale::*;
use commons::*;
use isometric::cell_traits::*;
use serde::{Deserialize, Serialize};
use std::default::Default;
use std::time::Duration;

#[derive(PartialEq, Debug, Serialize, Deserialize)]
pub struct AvatarTravelParams {
    pub max_walk_gradient: f32,
    pub walk_1_cell_duration_millis_range: (f32, f32),
    pub min_navigable_river_width: f32,
    pub max_navigable_river_gradient: f32,
    pub river_1_cell_duration_millis: f32,
    pub road_1_cell_duration_millis: u64,
    pub sea_1_cell_duration_millis: u64,
}

impl Default for AvatarTravelParams {
    fn default() -> AvatarTravelParams {
        AvatarTravelParams {
            max_walk_gradient: 0.5,
            walk_1_cell_duration_millis_range: (500.0, 1000.0),
            min_navigable_river_width: 0.1,
            max_navigable_river_gradient: 0.1,
            river_1_cell_duration_millis: 250.0,
            road_1_cell_duration_millis: 100,
            sea_1_cell_duration_millis: 250,
        }
    }
}

pub struct AvatarTravelDuration {
    travel_mode_fn: TravelModeFn,
    walk: Box<dyn TravelDuration>,
    road: Box<dyn TravelDuration>,
    river: Box<dyn TravelDuration>,
    sea: Box<dyn TravelDuration>,
}

impl AvatarTravelDuration {
    fn new(
        travel_mode_fn: TravelModeFn,
        walk: Box<dyn TravelDuration>,
        road: Box<dyn TravelDuration>,
        river: Box<dyn TravelDuration>,
        sea: Box<dyn TravelDuration>,
    ) -> AvatarTravelDuration {
        AvatarTravelDuration {
            travel_mode_fn,
            walk,
            road,
            river,
            sea,
        }
    }

    pub fn from_params(p: &AvatarTravelParams) -> AvatarTravelDuration {
        let walk = GradientTravelDuration::boxed(
            Scale::new(
                (-p.max_walk_gradient, p.max_walk_gradient),
                p.walk_1_cell_duration_millis_range,
            ),
            false,
        );
        let river = GradientTravelDuration::boxed(
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
        );
        let road =
            ConstantTravelDuration::boxed(Duration::from_millis(p.road_1_cell_duration_millis));
        let sea =
            ConstantTravelDuration::boxed(Duration::from_millis(p.sea_1_cell_duration_millis));
        AvatarTravelDuration::new(
            TravelModeFn::new(p.min_navigable_river_width),
            walk,
            road,
            river,
            sea,
        )
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
                TravelMode::River => self.river.as_ref(),
                TravelMode::Sea => self.sea.as_ref(),
            })
    }
}

impl TravelDuration for AvatarTravelDuration {
    fn get_duration(&self, world: &World, from: &V2<usize>, to: &V2<usize>) -> Option<Duration> {
        if let Some(cell) = world.get_cell(from) {
            if cell.is_visible() {
                return self
                    .get_duration_fn(world, from, to)
                    .and_then(|duration_fn| duration_fn.get_duration(world, from, to));
            }
        }
        None
    }

    fn max_duration(&self) -> Duration {
        self.walk
            .max_duration()
            .max(self.road.max_duration())
            .max(self.river.max_duration())
    }
}
