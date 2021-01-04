use commons::async_trait::async_trait;
use std::collections::HashMap;

use crate::avatar::Avatar;

#[async_trait]
pub trait SendAvatars {
    async fn send_avatars<F, O>(&self, function: F) -> O
    where
        O: Send + 'static,
        F: FnOnce(&mut HashMap<String, Avatar>) -> O + Send + 'static;
}
