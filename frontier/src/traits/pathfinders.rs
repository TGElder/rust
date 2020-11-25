use crate::traits::SendPathfinder;

pub trait PathfinderWithPlannedRoads {
    type T: SendPathfinder + Clone + Send + Sync;

    fn pathfinder_with_planned_roads(&self) -> &Self::T;
}

pub trait PathfinderWithoutPlannedRoads {
    type T: SendPathfinder + Clone + Send + Sync;

    fn pathfinder_without_planned_roads(&self) -> &Self::T;
}
