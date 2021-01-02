use commons::async_trait::async_trait;

use crate::actors::Micros;

#[async_trait]
pub trait SendMicros {
    async fn send_micros<F, O>(&self, function: F) -> O
    where
        O: Send + 'static,
        F: FnOnce(&mut Micros) -> O + Send + 'static;
}
