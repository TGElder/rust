use crate::simulation::build::edges::EdgeBuildSimulation;
use crate::traits::has::HasParameters;
use crate::traits::{
    InsertBuildInstruction, IsRoad, PlanRoad, RemoveBuildInstruction, RemoveRoad, RoadPlanned,
    WithBridges, WithEdgeTraffic, WithRoutes, WithWorld,
};
use crate::travel_duration::TravelDuration;
use commons::async_trait::async_trait;
use futures::future::BoxFuture;

#[async_trait]
pub trait SendEdgeBuildSim:
    HasParameters
    + InsertBuildInstruction
    + IsRoad
    + PlanRoad
    + RemoveBuildInstruction
    + RemoveRoad
    + RoadPlanned
    + WithBridges
    + WithEdgeTraffic
    + WithRoutes
    + WithWorld
    + Send
    + Sync
{
    type D: TravelDuration + 'static;

    async fn send_edge_build_sim_future<F, O>(&self, function: F) -> O
    where
        O: Send + 'static,
        F: FnOnce(&mut EdgeBuildSimulation<Self, Self::D>) -> BoxFuture<O> + Send + 'static;
}
