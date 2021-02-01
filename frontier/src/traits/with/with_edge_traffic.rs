use commons::async_trait::async_trait;

use crate::traffic::EdgeTraffic;

#[async_trait]
pub trait WithEdgeTraffic {
    async fn get_edge_traffic<F, O>(&self, function: F) -> O
    where
        F: FnOnce(&EdgeTraffic) -> O + Send;

    async fn mut_edge_traffic<F, O>(&self, function: F) -> O
    where
        F: FnOnce(&mut EdgeTraffic) -> O + Send;
}
