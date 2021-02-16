use commons::async_trait::async_trait;

use crate::actors::{Clock, Now};

#[async_trait]
pub trait WithClock {
    type T: Now;

    async fn with_clock<F, O>(&self, function: F) -> O
    where
        F: FnOnce(&Clock<Self::T>) -> O + Send;

    async fn mut_clock<F, O>(&self, function: F) -> O
    where
        F: FnOnce(&mut Clock<Self::T>) -> O + Send;
}
