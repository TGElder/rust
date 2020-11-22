use crate::actors::VisibilityActor;
use commons::async_trait::async_trait;

#[async_trait]
pub trait WithVisibility {
    async fn with_visibility<F, O>(&mut self, function: F) -> O
    where
        O: Send + 'static,
        F: FnOnce(&mut VisibilityActor) -> O + Send + 'static;

    fn with_visibility_background<F, O>(&mut self, function: F)
    where
        O: Send + 'static,
        F: FnOnce(&mut VisibilityActor) -> O + Send + 'static;
}