use commons::async_trait::async_trait;
use futures::future::BoxFuture;

use crate::simulation::build::positions::PositionBuildSimulation;
use crate::traits::has::HasParameters;
use crate::traits::{
    AnyoneControls, GetBuildInstruction, GetSettlement, InsertBuildInstruction, RandomTownName,
    RemoveBuildInstruction, RemoveWorldObject, WithRouteToPorts, WithRoutes, WithTraffic,
    WithWorld,
};

#[async_trait]
pub trait SendPositionBuildSim:
    AnyoneControls
    + GetBuildInstruction
    + GetSettlement
    + HasParameters
    + InsertBuildInstruction
    + RandomTownName
    + RemoveBuildInstruction
    + RemoveWorldObject
    + WithRoutes
    + WithRouteToPorts
    + WithTraffic
    + WithWorld
    + Send
    + Sync
{
    async fn send_position_build_sim_future<F, O>(&self, function: F) -> O
    where
        O: Send + 'static,
        F: FnOnce(&mut PositionBuildSimulation<Self>) -> BoxFuture<O> + Send + 'static;

    fn send_position_build_sim_future_background<F, O>(&self, function: F)
    where
        O: Send + 'static,
        F: FnOnce(&mut PositionBuildSimulation<Self>) -> BoxFuture<O> + Send + 'static;
}
