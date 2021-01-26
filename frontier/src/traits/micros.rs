use crate::traits::SendClock;
use commons::async_trait::async_trait;

#[async_trait]
pub trait Micros {
    async fn micros(&self) -> u128;
}

#[async_trait]
impl<T> Micros for T
where
    T: SendClock + Send + Sync + 'static,
{
    async fn micros(&self) -> u128 {
        self.send_clock(|micros| micros.get_micros()).await
    }
}
