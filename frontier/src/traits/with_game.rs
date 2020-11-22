use commons::async_trait::async_trait;

use crate::game::Game;

#[async_trait]
pub trait WithGame {
    async fn with_game<F, O>(&mut self, function: F) -> O
    where
        O: Send + 'static,
        F: FnOnce(&mut Game) -> O + Send + 'static;

    fn with_game_background<F, O>(&mut self, function: F)
    where
        O: Send + 'static,
        F: FnOnce(&mut Game) -> O + Send + 'static;
}
