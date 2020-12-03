use crate::settlement::Settlement;
use commons::async_trait::async_trait;
use commons::V2;
use std::collections::HashMap;

#[async_trait]
pub trait SendSettlements {
    async fn send_settlements<F, O>(&self, function: F) -> O
    where
        O: Send + 'static,
        F: FnOnce(&mut HashMap<V2<usize>, Settlement>) -> O + Send + 'static;
}
