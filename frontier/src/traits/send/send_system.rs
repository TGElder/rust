use commons::async_trait::async_trait;
use futures::future::BoxFuture;

use crate::system::System;

#[async_trait]
pub trait SendSystem {
    async fn send_system<F, O>(&self, function: F) -> O
    where
        O: Send + 'static,
        F: FnOnce(&mut System) -> O + Send + 'static;

    async fn send_system_future<F, O>(&self, function: F) -> O
    where
        O: Send + 'static,
        F: FnOnce(&mut System) -> BoxFuture<O> + Send + 'static;
}
