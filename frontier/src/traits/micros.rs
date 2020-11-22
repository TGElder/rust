use crate::traits::WithGame;
use commons::async_trait::async_trait;

#[async_trait]
pub trait Micros {
    async fn micros(&mut self) -> u128;
}

#[async_trait]
impl<T> Micros for T
where
    T: WithGame + Send + 'static,
{
    async fn micros(&mut self) -> u128 {
        self.with_game(|game| game.game_state().game_micros).await
    }
}
