use commons::async_trait::async_trait;

use crate::simulation::settlement::SettlementSimulation;

#[async_trait]
pub trait SendSettlementSim: Send {
    async fn send_settlement_sim<F, O>(&self, function: F) -> O
    where
        O: Send + 'static,
        F: FnOnce(&mut SettlementSimulation) -> O + Send + 'static;

    fn send_settlement_sim_background<F, O>(&self, function: F)
    where
        O: Send + 'static,
        F: FnOnce(&mut SettlementSimulation) -> O + Send + 'static;
}
