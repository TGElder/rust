use commons::async_trait::async_trait;

use crate::territory::Territory;

#[async_trait]
pub trait WithTerritory {
    async fn with_territory<F, O>(&self, function: F) -> O
    where
        F: FnOnce(&Territory) -> O + Send;

    async fn mut_territory<F, O>(&self, function: F) -> O
    where
        F: FnOnce(&mut Territory) -> O + Send;
}
