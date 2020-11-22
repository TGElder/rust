use commons::async_trait::async_trait;

use crate::polysender::Polysender;
use crate::world::World;

#[async_trait]
pub trait WithWorld {
    async fn with_world<F, O>(&mut self, function: F) -> O
    where
        O: Send + 'static,
        F: FnOnce(&mut World) -> O + Send + 'static;

    fn with_world_background<F, O>(&mut self, function: F)
    where
        O: Send + 'static,
        F: FnOnce(&mut World) -> O + Send + 'static;
}

#[async_trait]
impl WithWorld for Polysender {
    async fn with_world<F, O>(&mut self, function: F) -> O
    where
        O: Send + 'static,
        F: FnOnce(&mut World) -> O + Send + 'static,
    {
        self.game
            .send(move |game| function(&mut game.mut_state().world))
            .await
    }

    fn with_world_background<F, O>(&mut self, function: F)
    where
        O: Send + 'static,
        F: FnOnce(&mut World) -> O + Send + 'static,
    {
        self.game
            .send(move |game| function(&mut game.mut_state().world));
    }
}
