use commons::async_trait::async_trait;

use crate::territory::Controllers;

#[async_trait]
pub trait WithControllers {
    async fn with_controllers<F, O>(&self, function: F) -> O
    where
        F: FnOnce(&Controllers) -> O + Send;

    async fn mut_controllers<F, O>(&self, function: F) -> O
    where
        F: FnOnce(&mut Controllers) -> O + Send;
}
