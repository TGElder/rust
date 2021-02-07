use commons::async_trait::async_trait;

use crate::simulation::build::positions::PositionBuildSimulation;

#[async_trait]
pub trait SendPositionBuildSim: Send {
    fn send_position_build_sim_background<F, O>(&self, function: F)
    where
        O: Send + 'static,
        F: FnOnce(&mut PositionBuildSimulation) -> O + Send + 'static;
}
