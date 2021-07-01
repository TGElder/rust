use super::*;

use crate::world::World;
use commons::*;

pub trait TravelModeFn {
    fn travel_mode_between(
        &self,
        world: &World,
        from: &V2<usize>,
        to: &V2<usize>,
    ) -> Option<TravelMode>;

    fn travel_mode_here(&self, world: &World, position: &V2<usize>) -> Option<TravelMode>;
}
