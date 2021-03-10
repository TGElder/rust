use commons::async_trait::async_trait;

use crate::services::VisibilityService;

#[async_trait]
pub trait WithVisibility {
    async fn with_visibility<F, O>(&self, function: F) -> O
    where
        F: FnOnce(&VisibilityService) -> O + Send;

    async fn mut_visibility<F, O>(&self, function: F) -> O
    where
        F: FnOnce(&mut VisibilityService) -> O + Send;
}
