use std::borrow::Borrow;

use commons::V2;
use serde::{Deserialize, Serialize};

use crate::avatar::{AvatarTravelMode, TravelModeFn};
use crate::world::World;

#[derive(Clone, Copy, Debug, PartialEq, Deserialize, Serialize)]
pub enum Vehicle {
    None,
    Boat,
}

impl<T> From<T> for Vehicle
where
    T: Borrow<AvatarTravelMode>,
{
    fn from(mode: T) -> Self {
        match mode.borrow() {
            AvatarTravelMode::Walk => Vehicle::None,
            AvatarTravelMode::Road => Vehicle::None,
            AvatarTravelMode::PlannedRoad => Vehicle::None,
            AvatarTravelMode::Stream => Vehicle::None,
            AvatarTravelMode::River => Vehicle::Boat,
            AvatarTravelMode::Sea => Vehicle::Boat,
        }
    }
}

pub trait VehicleFn {
    fn vehicle_between(&self, world: &World, from: &V2<usize>, to: &V2<usize>) -> Option<Vehicle>;
}

impl<T> VehicleFn for T
where
    T: TravelModeFn,
{
    fn vehicle_between(&self, world: &World, from: &V2<usize>, to: &V2<usize>) -> Option<Vehicle> {
        self.travel_mode_between(world, from, to)
            .map(|mode| mode.into())
    }
}
