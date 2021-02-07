use commons::async_trait::async_trait;
use futures::future::BoxFuture;

use crate::simulation::build::edges::EdgeBuildSimulation;
use crate::traits::{
    InsertBuildInstruction, IsRoad, PlanRoad, RemoveBuildInstruction, RemoveRoad, RoadPlanned,
    SendRoutes, SendWorld, WithEdgeTraffic,
};
use crate::travel_duration::TravelDuration;

#[async_trait]
pub trait SendEdgeBuildSim:
    InsertBuildInstruction
    + IsRoad
    + PlanRoad
    + RemoveBuildInstruction
    + RemoveRoad
    + RoadPlanned
    + SendRoutes
    + SendWorld
    + WithEdgeTraffic
    + Send
    + Sync
{
    type D: TravelDuration + 'static;

    fn send_edge_build_sim_future_background<F, O>(&self, function: F)
    where
        O: Send + 'static,
        F: FnOnce(&mut EdgeBuildSimulation<Self, Self::D>) -> BoxFuture<O> + Send + 'static;
}
