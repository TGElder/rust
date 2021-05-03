use std::time::Duration;

use commons::async_trait::async_trait;
use commons::V2;

use crate::traits::has::HasTravelDurations;
use crate::traits::WithWorld;
use crate::travel_duration::TravelDuration;

#[async_trait]
pub trait CostOfPath {
    async fn cost_of_path<D>(&self, travel_duration: &D, path: &[V2<usize>]) -> Option<Duration>
    where
        D: TravelDuration;
}

#[async_trait]
impl<T> CostOfPath for T
where
    T: WithWorld + Sync,
{
    async fn cost_of_path<D>(&self, travel_duration: &D, path: &[V2<usize>]) -> Option<Duration>
    where
        D: TravelDuration,
    {
        self.with_world(|world| {
            (0..path.len() - 1)
                .map(|i| travel_duration.get_duration(world, &path[i], &path[i + 1]))
                .sum()
        })
        .await
    }
}

#[async_trait]
pub trait NpcCostOfPath {
    async fn npc_cost_of_path(&self, path: &[V2<usize>]) -> Option<Duration>;
}

#[async_trait]
impl<T> NpcCostOfPath for T
where
    T: CostOfPath + HasTravelDurations + Sync,
{
    async fn npc_cost_of_path(&self, path: &[V2<usize>]) -> Option<Duration> {
        self.cost_of_path(self.npc_travel_duration(), path).await
    }
}
