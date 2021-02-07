use commons::async_trait::async_trait;

use crate::simulation::build::edges::EdgeBuildSimulation;

#[async_trait]
pub trait SendEdgeBuildSim: Send {
    fn send_edge_build_sim_background<F, O>(&self, function: F)
    where
        O: Send + 'static,
        F: FnOnce(&mut EdgeBuildSimulation) -> O + Send + 'static;
}
