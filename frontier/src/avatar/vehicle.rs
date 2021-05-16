use std::borrow::Borrow;

use commons::V2;
use serde::{Deserialize, Serialize};

use crate::avatar::{TravelMode, TravelModeFn};
use crate::world::World;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Deserialize, Serialize)]
pub enum Vehicle {
    None,
    Boat,
}

impl<T> From<T> for Vehicle
where
    T: Borrow<TravelMode>,
{
    fn from(mode: T) -> Self {
        match mode.borrow() {
            TravelMode::Walk => Vehicle::None,
            TravelMode::Road => Vehicle::None,
            TravelMode::PlannedRoad => Vehicle::None,
            TravelMode::Stream => Vehicle::None,
            TravelMode::River => Vehicle::Boat,
            TravelMode::Sea => Vehicle::Boat,
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
