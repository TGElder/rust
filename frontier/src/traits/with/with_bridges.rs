use commons::async_trait::async_trait;

use crate::bridges::Bridges;

#[async_trait]
pub trait WithBridges {
    async fn with_bridges<F, O>(&self, function: F) -> O
    where
        F: FnOnce(&Bridges) -> O + Send;

    async fn mut_bridges<F, O>(&self, function: F) -> O
    where
        F: FnOnce(&mut Bridges) -> O + Send;
}
