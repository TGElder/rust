use commons::async_trait::async_trait;

use crate::avatars::Avatars;

#[async_trait]
pub trait WithAvatars {
    async fn with_avatars<F, O>(&self, function: F) -> O
    where
        F: FnOnce(&Avatars) -> O + Send;

    async fn mut_avatars<F, O>(&self, function: F) -> O
    where
        F: FnOnce(&mut Avatars) -> O + Send;
}
