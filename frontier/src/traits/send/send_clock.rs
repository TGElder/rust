use commons::async_trait::async_trait;

use crate::actors::{Clock, Now};

#[async_trait]
pub trait SendClock {
    type T: Now;

    async fn send_clock<F, O>(&self, function: F) -> O
    where
        O: Send + 'static,
        F: FnOnce(&mut Clock<Self::T>) -> O + Send + 'static;
}
