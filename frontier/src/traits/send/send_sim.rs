use commons::async_trait::async_trait;
use commons::fn_sender::FnSender;

use crate::simulation::Simulation;

#[async_trait]
pub trait SendSim {
    async fn send_sim<F, O>(&self, function: F) -> O
    where
        O: Send + 'static,
        F: FnOnce(&mut Simulation) -> O + Send + 'static;

    fn send_sim_background<F, O>(&self, function: F)
    where
        O: Send + 'static,
        F: FnOnce(&mut Simulation) -> O + Send + 'static;
}

#[async_trait]
impl SendSim for FnSender<Simulation> {
    async fn send_sim<F, O>(&self, function: F) -> O
    where
        O: Send + 'static,
        F: FnOnce(&mut Simulation) -> O + Send + 'static,
    {
        self.send(function).await
    }

    fn send_sim_background<F, O>(&self, function: F)
    where
        O: Send + 'static,
        F: FnOnce(&mut Simulation) -> O + Send + 'static,
    {
        self.send(function);
    }
}
