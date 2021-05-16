use std::time::Duration;

use commons::async_trait::async_trait;
use commons::V2;

use crate::pathfinder::ClosestTargetResult;
use crate::traits::{
    ClosestTargets, CostOfPath, InBounds, InitTargets, LoadTargets, Target, WithBridges,
    WithPathfinder, WithWorld,
};

pub trait PathfinderForRoutes {
    type T: WithPathfinder + Clone + Send + Sync + 'static;

    fn routes_pathfinder(&self) -> &Self::T;
}

#[async_trait]
pub trait InBoundsForRoutes {
    async fn in_bounds(&self, position: &V2<usize>) -> bool;
}

#[async_trait]
impl<T> InBoundsForRoutes for T
where
    T: PathfinderForRoutes + Sync,
{
    async fn in_bounds(&self, position: &V2<usize>) -> bool {
        self.routes_pathfinder().in_bounds(position).await
    }
}

#[async_trait]
pub trait InitTargetsForRoutes {
    async fn init_targets(&self, name: String);
}

#[async_trait]
impl<T> InitTargetsForRoutes for T
where
    T: PathfinderForRoutes + Sync,
{
    async fn init_targets(&self, name: String) {
        self.routes_pathfinder().init_targets(name).await;
    }
}

#[async_trait]
pub trait LoadTargetForRoutes {
    async fn load_targets<'a, I>(&self, targets: I)
    where
        I: Iterator<Item = Target<'a>> + Send;
}

#[async_trait]
impl<T> LoadTargetForRoutes for T
where
    T: PathfinderForRoutes + Sync,
{
    async fn load_targets<'a, I>(&self, targets: I)
    where
        I: Iterator<Item = Target<'a>> + Send,
    {
        self.routes_pathfinder().load_targets(targets).await
    }
}

#[async_trait]
pub trait ClosestTargetsForRoutes {
    async fn closest_targets(
        &self,
        positions: &[V2<usize>],
        targets: &str,
        n_closest: usize,
    ) -> Vec<ClosestTargetResult>;
}

#[async_trait]
impl<T> ClosestTargetsForRoutes for T
where
    T: PathfinderForRoutes + Sync,
{
    async fn closest_targets(
        &self,
        positions: &[V2<usize>],
        targets: &str,
        n_closest: usize,
    ) -> Vec<ClosestTargetResult> {
        self.routes_pathfinder()
            .closest_targets(positions, targets, n_closest)
            .await
    }
}

pub trait PathfinderForPlayer {
    type T: WithPathfinder + Clone + Send + Sync + 'static;

    fn player_pathfinder(&self) -> &Self::T;
}

#[async_trait]
pub trait CostOfPathForPlayer {
    async fn cost_of_path_for_player(&self, path: &[V2<usize>]) -> Option<Duration>;
}

#[async_trait]
impl<T> CostOfPathForPlayer for T
where
    T: PathfinderForPlayer + WithBridges + WithWorld + Sync,
{
    async fn cost_of_path_for_player(&self, path: &[V2<usize>]) -> Option<Duration> {
        let travel_duration = self
            .player_pathfinder()
            .with_pathfinder(|pathfinder| pathfinder.travel_duration().clone())
            .await;

        let bridges = self.with_bridges(|bridges| (*bridges).clone()).await;

        self.cost_of_path(travel_duration.as_ref(), &bridges, path)
            .await
    }
}
