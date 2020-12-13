use commons::async_trait::async_trait;

use crate::simulation::Simulation;
use crate::traits::SendWorld;

#[async_trait]
pub trait SendSim: SendWorld + Send {
    async fn send_sim<F, O>(&self, function: F) -> O
    where
        O: Send + 'static,
        F: FnOnce(&mut Simulation<Self>) -> O + Send + 'static;

    fn send_sim_background<F, O>(&self, function: F)
    where
        O: Send + 'static,
        F: FnOnce(&mut Simulation<Self>) -> O + Send + 'static;
}

// #[async_trait]
// impl <X> SendSim for FnSender<Simulation<X>>
//     where X: Send
// {
//     async fn send_sim<F, O>(&self, function: F) -> O
//     where
//         O: Send + 'static,
//         F: FnOnce(&mut Simulation<X>) -> O + Send + 'static,
//     {
//         self.send(function).await
//     }

//     fn send_sim_background<F, O>(&self, function: F)
//     where
//         O: Send + 'static,
//         F: FnOnce(&mut Simulation<X>) -> O + Send + 'static,
//     {
//         self.send(function);
//     }
// }
