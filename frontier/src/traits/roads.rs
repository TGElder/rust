use commons::async_trait::async_trait;
use commons::edge::Edge;

use crate::road_builder::{RoadBuildMode, RoadBuilderResult};
use crate::traits::{
    NotMock, PathfinderForRouting, UpdatePathfinderPositions, UpdateRoads, WithWorld,
};

#[async_trait]
pub trait IsRoad {
    async fn is_road(&self, edge: &Edge) -> bool;
}

#[async_trait]
impl<T> IsRoad for T
where
    T: WithWorld + Send + Sync,
{
    async fn is_road(&self, edge: &Edge) -> bool {
        self.with_world(|world| world.is_road(edge)).await
    }
}

#[async_trait]
pub trait AddRoad {
    async fn add_roads(&self, edge: &[Edge]);
}

#[async_trait]
impl<T> AddRoad for T
where
    T: WithWorld + UpdateRoads + Send + Sync,
{
    async fn add_roads(&self, edges: &[Edge]) {
        let to_build = self
            .with_world(|world| {
                edges
                    .iter()
                    .filter(|edge| !world.is_road(edge))
                    .copied()
                    .collect()
            })
            .await;
        let result = RoadBuilderResult::new(to_build, RoadBuildMode::Build);
        self.update_roads(result).await;
    }
}

#[async_trait]
pub trait RemoveRoad {
    async fn remove_roads(&self, edges: &[Edge]);
}

#[async_trait]
impl<T> RemoveRoad for T
where
    T: WithWorld + UpdateRoads + Send + Sync,
{
    async fn remove_roads(&self, edges: &[Edge]) {
        let to_remove = self
            .with_world(|world| {
                edges
                    .iter()
                    .filter(|edge| world.is_road(edge))
                    .copied()
                    .collect()
            })
            .await;
        let result = RoadBuilderResult::new(to_remove, RoadBuildMode::Demolish);
        self.update_roads(result).await;
    }
}

#[async_trait]
pub trait RoadPlanned {
    async fn road_planned(&self, edge: &Edge) -> Option<u128>;
}

#[async_trait]
impl<T> RoadPlanned for T
where
    T: WithWorld + NotMock + Send + Sync,
{
    async fn road_planned(&self, edge: &Edge) -> Option<u128> {
        self.with_world(|world| world.road_planned(edge)).await
    }
}
#[async_trait]
pub trait PlanRoad {
    async fn plan_road(&self, edge: &Edge, when: Option<u128>);
}

#[async_trait]
impl<T> PlanRoad for T
where
    T: PathfinderForRouting + UpdatePathfinderPositions + WithWorld + Send + Sync,
{
    async fn plan_road(&self, edge: &Edge, when: Option<u128>) {
        self.mut_world(|world| world.plan_road(edge, when)).await;
        self.update_pathfinder_positions(self.routing_pathfinder(), vec![*edge.from(), *edge.to()])
            .await;
    }
}
