use commons::async_trait::async_trait;
use commons::edge::Edge;
use std::collections::HashSet;

use crate::road_builder::{RoadBuildMode, RoadBuilderResult};
use crate::traits::UpdateRoads;

use crate::traits::IsRoad;

#[async_trait]
pub trait BuildRoads {
    async fn add_road(&mut self, edge: &Edge);

    async fn remove_road(&mut self, edge: &Edge);
}

#[async_trait]
impl<T> BuildRoads for T
where
    T: IsRoad + UpdateRoads + Send,
{
    async fn add_road(&mut self, edge: &Edge) {
        if self.is_road(*edge).await {
            return;
        }
        let result = RoadBuilderResult::new(vec![*edge.from(), *edge.to()], RoadBuildMode::Build);
        self.update_roads(result).await;
    }

    async fn remove_road(&mut self, edge: &Edge) {
        if self.is_road(*edge).await {
            return;
        }
        let result =
            RoadBuilderResult::new(vec![*edge.from(), *edge.to()], RoadBuildMode::Demolish);
        self.update_roads(result).await;
    }
}

#[async_trait]
impl BuildRoads for HashSet<Edge> {
    async fn add_road(&mut self, edge: &Edge) {
        self.insert(*edge);
    }

    async fn remove_road(&mut self, edge: &Edge) {
        self.remove(edge);
    }
}