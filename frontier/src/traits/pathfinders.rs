use std::sync::Arc;

use crate::traits::SendPathfinder;
use crate::travel_duration::TravelDuration;

pub trait PathfinderWithoutPlannedRoads<T>
    where T: TravelDuration
{

    fn travel_duration_without_planned_roads(&mut self) -> &Arc<T>;
    fn pathfinder_without_planned_roads(&mut self) -> Box<dyn SendPathfinder<T>>;
}

pub trait PathfinderWithPlannedRoads<T>
    where T: TravelDuration
{
    fn travel_duration_with_planned_roads(&mut self) -> &Arc<T>;
    fn pathfinder_with_planned_roads(&mut self) -> Box<dyn SendPathfinder<T>>;
}