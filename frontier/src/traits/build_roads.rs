use commons::async_trait::async_trait;
use commons::edge::Edge;
use commons::Arm;
use std::collections::HashSet;

use crate::road_builder::{RoadBuildMode, RoadBuilderResult};
use crate::traits::UpdateRoads;

use crate::traits::IsRoad;

#[async_trait]
pub trait BuildRoads {
    async fn add_road(&self, edge: &Edge);

    async fn remove_road(&self, edge: &Edge);
}

#[async_trait]
impl<T> BuildRoads for T
where
    T: IsRoad + UpdateRoads + Send + Sync,
{
    async fn add_road(&self, edge: &Edge) {
        if self.is_road(*edge).await {
            return;
        }
        let result = RoadBuilderResult::new(vec![*edge.from(), *edge.to()], RoadBuildMode::Build);
        self.update_roads(result).await;
    }

    async fn remove_road(&self, edge: &Edge) {
        if self.is_road(*edge).await {
            return;
        }
        let result =
            RoadBuilderResult::new(vec![*edge.from(), *edge.to()], RoadBuildMode::Demolish);
        self.update_roads(result).await;
    }
}

#[async_trait]
impl BuildRoads for Arm<HashSet<Edge>> {
    async fn add_road(&self, edge: &Edge) {
        self.lock().unwrap().insert(*edge);
    }

    async fn remove_road(&self, edge: &Edge) {
        self.lock().unwrap().remove(edge);
    }
}
