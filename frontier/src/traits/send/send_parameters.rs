use crate::parameters::Parameters;
use commons::async_trait::async_trait;

#[async_trait]
pub trait SendParameters {
    async fn send_parameters<F, O>(&self, function: F) -> O
    where
        O: Send + 'static,
        F: FnOnce(&Parameters) -> O + Send + 'static;
}
