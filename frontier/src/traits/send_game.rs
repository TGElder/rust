use commons::async_trait::async_trait;

use crate::game::Game;

#[async_trait]
pub trait SendGame {
    async fn send_game<F, O>(&mut self, function: F) -> O
    where
        O: Send + 'static,
        F: FnOnce(&mut Game) -> O + Send + 'static;

    fn send_game_background<F, O>(&mut self, function: F)
    where
        O: Send + 'static,
        F: FnOnce(&mut Game) -> O + Send + 'static;
}
