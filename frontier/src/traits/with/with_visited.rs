use commons::async_trait::async_trait;

use crate::visited::Visited;

#[async_trait]
pub trait WithVisited {
    async fn with_visited<F, O>(&self, function: F) -> O
    where
        F: FnOnce(&Visited) -> O + Send;

    async fn mut_visited<F, O>(&self, function: F) -> O
    where
        F: FnOnce(&mut Visited) -> O + Send;
}
