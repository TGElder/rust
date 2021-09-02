use commons::async_trait::async_trait;
use futures::future::BoxFuture;

use crate::simulation::build::positions::PositionBuildSimulation;
use crate::traits::has::HasParameters;
use crate::traits::{
    AnyoneControls, GetSettlement, InsertBuildInstruction, Micros, RandomTownName,
    WithRouteToGates, WithRoutes, WithTraffic, WithWorld,
};

#[async_trait]
pub trait SendPositionBuildSim:
    AnyoneControls
    + GetSettlement
    + HasParameters
    + InsertBuildInstruction
    + Micros
    + RandomTownName
    + WithRoutes
    + WithRouteToGates
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
