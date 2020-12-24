use commons::async_trait::async_trait;
use commons::edge::Edge;

use crate::road_builder::{RoadBuildMode, RoadBuilderResult};
use crate::traits::{
    NotMock, PathfinderWithPlannedRoads, SendWorld, UpdatePathfinderPositions, UpdateRoads,
};

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

#[async_trait]
pub trait RoadPlanned {
    async fn road_planned(&self, edge: Edge) -> Option<u128>;
}

#[async_trait]
impl<T> RoadPlanned for T
where
    T: SendWorld + NotMock + Send + Sync,
{
    async fn road_planned(&self, edge: Edge) -> Option<u128> {
        self.send_world(move |world| world.road_planned(&edge))
            .await
    }
}
#[async_trait]
pub trait PlanRoad {
    async fn plan_road(&self, edge: Edge, when: Option<u128>);
}

#[async_trait]
impl<T> PlanRoad for T
where
    T: PathfinderWithPlannedRoads + SendWorld + Send + Sync,
{
    async fn plan_road(&self, edge: Edge, when: Option<u128>) {
        self.send_world(move |world| world.plan_road(&edge, when))
            .await;
        let pathfinder = self.pathfinder_with_planned_roads().clone();
        self.update_pathfinder_positions(pathfinder, vec![*edge.from(), *edge.to()])
            .await;
    }
}
