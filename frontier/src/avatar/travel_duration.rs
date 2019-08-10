use super::travel_mode::*;
use crate::travel_duration::*;
use crate::world::World;
use commons::*;
use isometric::cell_traits::*;
use std::time::Duration;

pub struct AvatarTravelDuration {
    travel_mode_fn: TravelModeFn,
    walk: Box<TravelDuration>,
    road: Box<TravelDuration>,
    river: Box<TravelDuration>,
    sea: Box<TravelDuration>,
}

impl AvatarTravelDuration {
    pub fn new(
        travel_mode_fn: TravelModeFn,
        walk: Box<TravelDuration>,
        road: Box<TravelDuration>,
        river: Box<TravelDuration>,
        sea: Box<TravelDuration>,
    ) -> AvatarTravelDuration {
        AvatarTravelDuration {
            travel_mode_fn,
            walk,
            road,
            river,
            sea,
        }
    }

    pub fn boxed(
        travel_mode_fn: TravelModeFn,
        walk: Box<TravelDuration>,
        road: Box<TravelDuration>,
        river: Box<TravelDuration>,
        sea: Box<TravelDuration>,
    ) -> Box<TravelDuration> {
        Box::new(AvatarTravelDuration::new(
            travel_mode_fn,
            walk,
            road,
            river,
            sea,
        ))
    }
}

impl AvatarTravelDuration {
    fn get_duration_fn(
        &self,
        world: &World,
        from: &V2<usize>,
        to: &V2<usize>,
    ) -> Option<&TravelDuration> {
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
