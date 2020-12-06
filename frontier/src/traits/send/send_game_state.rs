use crate::game::GameState;
use commons::async_trait::async_trait;

#[async_trait]
pub trait SendGameState {
    async fn send_game_state<F, O>(&self, function: F) -> O
    where
        O: Send + 'static,
        F: FnOnce(&mut GameState) -> O + Send + 'static;
}
