use commons::async_trait::async_trait;
use commons::V2;

#[async_trait]
pub trait WithSimQueue {
    async fn with_sim_queue<F, O>(&self, function: F) -> O
    where
        F: FnOnce(&Vec<V2<usize>>) -> O + Send;

    async fn mut_sim_queue<F, O>(&self, function: F) -> O
    where
        F: FnOnce(&mut Vec<V2<usize>>) -> O + Send;
}
