use crate::traits::send::SendGame;
use commons::async_trait::async_trait;

#[async_trait]
pub trait Micros {
    async fn micros(&self) -> u128;
}

#[async_trait]
impl<T> Micros for T
where
    T: SendGame + Send + Sync + 'static,
{
    async fn micros(&self) -> u128 {
        self.send_game(|game| game.game_state().game_micros).await
    }
}
