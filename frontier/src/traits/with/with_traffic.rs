use commons::async_trait::async_trait;

use crate::traffic::Traffic;

#[async_trait]
pub trait WithTraffic {
    async fn get_traffic<F, O>(&self, function: F) -> O
    where
        F: FnOnce(&Traffic) -> O + Send;

    async fn mut_traffic<F, O>(&self, function: F) -> O
    where
        F: FnOnce(&mut Traffic) -> O + Send;
}
