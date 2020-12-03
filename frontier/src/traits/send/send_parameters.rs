use crate::game::GameParams;
use commons::async_trait::async_trait;

#[async_trait]
pub trait SendParameters {
    async fn send_parameters<F, O>(&self, function: F) -> O
    where
        O: Send + 'static,
        F: FnOnce(&GameParams) -> O + Send + 'static;
}
