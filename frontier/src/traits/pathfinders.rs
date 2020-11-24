use std::sync::Arc;

use crate::traits::SendPathfinder;
use crate::travel_duration::TravelDuration;

pub trait PathfinderWithoutPlannedRoads {
    type T: TravelDuration + 'static;
    type U: SendPathfinder<Self::T> + Clone + Send + Sync;

    fn travel_duration_without_planned_roads(&self) -> &Arc<Self::T>;
    fn pathfinder_without_planned_roads(&self) -> &Self::U;
}

// pub trait PathfinderWithPlannedRoads
// {
//     fn travel_duration_with_planned_roadss<T: TravelDuration> (&mut self) -> &Arc<T>;
//     fn pathfinder_with_planned_roads<T: TravelDuration, U: SendPathfinder<T>>(&mut self) -> U;
// }
