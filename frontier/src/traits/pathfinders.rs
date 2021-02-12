use std::time::Duration;

use commons::async_trait::async_trait;
use commons::V2;

use crate::pathfinder::ClosestTargetResult;
use crate::traits::{
    ClosestTargets, InBounds, InitTargets, LoadTarget, LowestDuration, WithPathfinder,
};

pub trait PathfinderWithPlannedRoads {
    type T: WithPathfinder + Clone + Send + Sync;

    fn pathfinder_with_planned_roads(&self) -> &Self::T;
}

#[async_trait]
pub trait InBoundsWithPlannedRoads {
    async fn in_bounds(&self, position: &V2<usize>) -> bool;
}

#[async_trait]
impl<T> InBoundsWithPlannedRoads for T
where
    T: PathfinderWithPlannedRoads + Sync,
{
    async fn in_bounds(&self, position: &V2<usize>) -> bool {
        self.pathfinder_with_planned_roads()
            .in_bounds(position)
            .await
    }
}

#[async_trait]
pub trait InitTargetsWithPlannedRoads {
    async fn init_targets(&self, name: String);
}

#[async_trait]
impl<T> InitTargetsWithPlannedRoads for T
where
    T: PathfinderWithPlannedRoads + Sync,
{
    async fn init_targets(&self, name: String) {
        self.pathfinder_with_planned_roads()
            .init_targets(name)
            .await;
    }
}

#[async_trait]
pub trait LoadTargetWithPlannedRoads {
    async fn load_target(&self, name: &str, position: &V2<usize>, target: bool);
}

#[async_trait]
impl<T> LoadTargetWithPlannedRoads for T
where
    T: PathfinderWithPlannedRoads + Sync,
{
    async fn load_target(&self, name: &str, position: &V2<usize>, target: bool) {
        self.pathfinder_with_planned_roads()
            .load_target(name, position, target)
            .await
    }
}

#[async_trait]
pub trait ClosestTargetsWithPlannedRoads {
    async fn closest_targets(
        &self,
        positions: &[V2<usize>],
        targets: &str,
        n_closest: usize,
    ) -> Vec<ClosestTargetResult>;
}

#[async_trait]
impl<T> ClosestTargetsWithPlannedRoads for T
where
    T: PathfinderWithPlannedRoads + Sync,
{
    async fn closest_targets(
        &self,
        positions: &[V2<usize>],
        targets: &str,
        n_closest: usize,
    ) -> Vec<ClosestTargetResult> {
        self.pathfinder_with_planned_roads()
            .closest_targets(positions, targets, n_closest)
            .await
    }
}

pub trait PathfinderWithoutPlannedRoads {
    type T: WithPathfinder + Clone + Send + Sync;

    fn pathfinder_without_planned_roads(&self) -> &Self::T;
}

#[async_trait]
pub trait LowestDurationWithoutPlannedRoads {
    async fn lowest_duration(&self, path: &[V2<usize>]) -> Option<Duration>;
}

#[async_trait]
impl<T> LowestDurationWithoutPlannedRoads for T
where
    T: PathfinderWithoutPlannedRoads + Sync,
{
    async fn lowest_duration(&self, path: &[V2<usize>]) -> Option<Duration> {
        self.pathfinder_without_planned_roads()
            .lowest_duration(path)
            .await
    }
}
