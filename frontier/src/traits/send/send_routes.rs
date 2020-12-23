use crate::route::Routes;
use commons::async_trait::async_trait;

#[async_trait]
pub trait SendRoutes {
    async fn send_routes<F, O>(&self, function: F) -> O
    where
        O: Send + 'static,
        F: FnOnce(&mut Routes) -> O + Send + 'static;
}
