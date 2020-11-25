use crate::traits::SendPathfinder;
use crate::travel_duration::TravelDuration;

pub trait PathfinderWithoutPlannedRoads {
    type T: TravelDuration + 'static;
    type U: SendPathfinder<Self::T> + Clone + Send + Sync;

    fn pathfinder_without_planned_roads(&self) -> &Self::U;
}

pub trait PathfinderWithPlannedRoads {
    type T: TravelDuration + 'static;
    type U: SendPathfinder<Self::T> + Clone + Send + Sync;

    fn pathfinder_with_planned_roads(&self) -> &Self::U;
}
