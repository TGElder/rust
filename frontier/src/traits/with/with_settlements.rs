use std::collections::HashMap;

use commons::async_trait::async_trait;
use commons::V2;

use crate::settlement::Settlement;

#[async_trait]
pub trait WithSettlements {
    async fn with_settlements<F, O>(&self, function: F) -> O
    where
        F: FnOnce(&HashMap<V2<usize>, Settlement>) -> O + Send;

    async fn mut_settlements<F, O>(&self, function: F) -> O
    where
        F: FnOnce(&mut HashMap<V2<usize>, Settlement>) -> O + Send;
}
