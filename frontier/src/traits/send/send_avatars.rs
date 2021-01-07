use commons::async_trait::async_trait;

use crate::avatars::Avatars;

#[async_trait]
pub trait SendAvatars {
    async fn send_avatars<F, O>(&self, function: F) -> O
    where
        O: Send + 'static,
        F: FnOnce(&mut Avatars) -> O + Send + 'static;

    fn send_avatars_background<F, O>(&self, function: F)
    where
        O: Send + 'static,
        F: FnOnce(&mut Avatars) -> O + Send + 'static;
}
