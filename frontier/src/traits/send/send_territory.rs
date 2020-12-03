use crate::territory::Territory;
use commons::async_trait::async_trait;


#[async_trait]
pub trait SendTerritory {
    async fn send_territory<F, O>(&self, function: F) -> O
    where
        O: Send + 'static,
        F: FnOnce(&mut Territory) -> O + Send + 'static;
}
