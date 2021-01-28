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

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    #[async_trait]
    impl SendSettlements for Mutex<HashMap<V2<usize>, Settlement>> {
        async fn send_settlements<F, O>(&self, function: F) -> O
        where
            O: Send + 'static,
            F: FnOnce(&mut HashMap<V2<usize>, Settlement>) -> O + Send + 'static,
        {
            function(&mut self.lock().unwrap())
        }
    }
}
