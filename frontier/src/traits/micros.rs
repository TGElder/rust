use crate::traits::SendMicros;
use commons::async_trait::async_trait;

#[async_trait]
pub trait Micros {
    async fn micros(&self) -> u128;
}

#[async_trait]
impl<T> Micros for T
where
    T: SendMicros + Send + Sync + 'static,
{
    async fn micros(&self) -> u128 {
        self.send_micros(|micros| micros.get_micros()).await
    }
}

#[async_trait]
pub trait SetSpeed {
    async fn set_speed(&self, speed: f32);
}

#[async_trait]
impl<T> SetSpeed for T
where
    T: SendMicros + Send + Sync + 'static,
{
    async fn set_speed(&self, speed: f32) {
        self.send_micros(move |micros| micros.set_speed(speed))
            .await
    }
}
