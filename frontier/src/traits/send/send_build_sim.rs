use commons::async_trait::async_trait;

use crate::build_sim::BuildSimulation;

#[async_trait]
pub trait SendBuildSim: Send {
    fn send_build_sim_background<F, O>(&self, function: F)
    where
        O: Send + 'static,
        F: FnOnce(&mut BuildSimulation) -> O + Send + 'static;
}
