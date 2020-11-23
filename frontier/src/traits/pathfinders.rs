use crate::traits::SendPathfinder;
use crate::travel_duration::TravelDuration;

pub trait PathfinderWithoutPlannedRoads<T, U>
    where T: TravelDuration,
    U: SendPathfinder<T>
{

    fn pathfinder_without_planned_roads(&mut self) -> U;
}

pub trait PathfinderWithPlannedRoads<T, U>
    where T: TravelDuration,
    U: SendPathfinder<T>
{

    fn pathfinder_with_planned_roads(&mut self) -> U;
}