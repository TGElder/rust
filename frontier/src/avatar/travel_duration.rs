use super::travel_mode::*;
use crate::travel_duration::*;
use crate::world::World;
use commons::V2;
use std::time::Duration;

pub struct AvatarTravelDuration {
    travel_mode_fn: TravelModeFn,
    walk: Box<TravelDuration>,
    road: Box<TravelDuration>,
    water: Box<TravelDuration>,
}

impl AvatarTravelDuration {
    pub fn new(
        travel_mode_fn: TravelModeFn,
        walk: Box<TravelDuration>,
        road: Box<TravelDuration>,
        water: Box<TravelDuration>,
    ) -> AvatarTravelDuration {
        AvatarTravelDuration {
            travel_mode_fn,
            walk,
            road,
            water,
        }
    }

    pub fn boxed(
        travel_mode_fn: TravelModeFn,
        walk: Box<TravelDuration>,
        road: Box<TravelDuration>,
        water: Box<TravelDuration>,
    ) -> Box<TravelDuration> {
        Box::new(AvatarTravelDuration::new(travel_mode_fn, walk, road, water))
    }
}

impl AvatarTravelDuration {
    fn get_duration_fn(
        &self,
        world: &World,
        from: &V2<usize>,
        to: &V2<usize>,
    ) -> Option<&Box<TravelDuration>> {
        self.travel_mode_fn
            .travel_mode_between(world, from, to)
            .map(|travel_mode| match travel_mode {
                TravelMode::Sea => &self.water,
                TravelMode::River => &self.water,
                TravelMode::Walk => &self.walk,
                TravelMode::Road => &self.road,
            })
    }
}

impl TravelDuration for AvatarTravelDuration {
    fn get_duration(&self, world: &World, from: &V2<usize>, to: &V2<usize>) -> Option<Duration> {
        self.get_duration_fn(world, from, to)
            .and_then(|duration_fn| duration_fn.get_duration(world, from, to))
    }

    fn max_duration(&self) -> Duration {
        self.walk
            .max_duration()
            .max(self.road.max_duration())
            .max(self.water.max_duration())
    }
}
