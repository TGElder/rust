use commons::async_trait::async_trait;
use commons::edge::Edge;

use crate::road_builder::{RoadBuildMode, RoadBuilderResult};
use crate::traits::{SendWorld, UpdateRoads};

#[async_trait]
pub trait IsRoad {
    async fn is_road(&self, edge: Edge) -> bool;
}

#[async_trait]
impl<T> IsRoad for T
where
    T: SendWorld + Send + Sync,
{
    async fn is_road(&self, edge: Edge) -> bool {
        self.send_world(move |world| world.is_road(&edge)).await
    }
}

#[async_trait]
pub trait AddRoad {
    async fn add_road(&self, edge: Edge);
}

#[async_trait]
impl<T> AddRoad for T
where
    T: IsRoad + UpdateRoads + Send + Sync,
{
    async fn add_road(&self, edge: Edge) {
        if self.is_road(edge).await {
            return;
        }
        let result = RoadBuilderResult::new(vec![*edge.from(), *edge.to()], RoadBuildMode::Build);
        self.update_roads(result).await;
    }
}

#[async_trait]
pub trait RemoveRoad {
    async fn remove_road(&self, edge: Edge);
}

#[async_trait]
impl<T> RemoveRoad for T
where
    T: IsRoad + UpdateRoads + Send + Sync,
{
    async fn remove_road(&self, edge: Edge) {
        if !self.is_road(edge).await {
            return;
        }
        let result =
            RoadBuilderResult::new(vec![*edge.from(), *edge.to()], RoadBuildMode::Demolish);
        self.update_roads(result).await;
    }
}
