use commons::async_trait::async_trait;

use crate::simulation::Simulation;
use crate::traits::SendWorld;

#[async_trait]
pub trait SendSim: SendWorld + Send {
    async fn send_sim<F, O>(&self, function: F) -> O
    where
        O: Send + 'static,
        F: FnOnce(&mut Simulation) -> O + Send + 'static;

    fn send_sim_background<F, O>(&self, function: F)
    where
        O: Send + 'static,
        F: FnOnce(&mut Simulation) -> O + Send + 'static;
}
