use commons::async_trait::async_trait;

use crate::resource::Resources;

#[async_trait]
pub trait WithResources {
    async fn with_resources<F, O>(&self, function: F) -> O
    where
        F: FnOnce(&Resources) -> O + Send;

    async fn mut_resources<F, O>(&self, function: F) -> O
    where
        F: FnOnce(&mut Resources) -> O + Send;
}
