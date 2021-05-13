use commons::async_trait::async_trait;

use crate::bridge::Bridges;

#[async_trait]
pub trait WithBridge {
    async fn with_bridges<F, O>(&self, function: F) -> O
    where
        F: FnOnce(&Bridges) -> O + Send;

    async fn mut_bridges<F, O>(&self, function: F) -> O
    where
        F: FnOnce(&mut Bridges) -> O + Send;
}
