use commons::async_trait::async_trait;

use crate::build::BuildQueue;

#[async_trait]
pub trait WithBuildQueue {
    async fn with_build_queue<F, O>(&self, function: F) -> O
    where
        F: FnOnce(&BuildQueue) -> O + Send;

    async fn mut_build_queue<F, O>(&self, function: F) -> O
    where
        F: FnOnce(&mut BuildQueue) -> O + Send;
}
