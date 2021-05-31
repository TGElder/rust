use commons::async_trait::async_trait;

use crate::territory::HomelandTerritory;

#[async_trait]
pub trait WithHomelandTerritory {
    async fn with_homeland_territory<F, O>(&self, function: F) -> O
    where
        F: FnOnce(&HomelandTerritory) -> O + Send;

    async fn mut_homeland_territory<F, O>(&self, function: F) -> O
    where
        F: FnOnce(&mut HomelandTerritory) -> O + Send;
}
