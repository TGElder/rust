use crate::world::World;
use commons::async_trait::async_trait;

#[async_trait]
pub trait SendWorld {
    async fn send_world<F, O>(&mut self, function: F) -> O
    where
        O: Send + 'static,
        F: FnOnce(&mut World) -> O + Send + 'static;

    fn send_world_background<F, O>(&mut self, function: F)
    where
        O: Send + 'static,
        F: FnOnce(&mut World) -> O + Send + 'static;
}
