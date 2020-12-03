use commons::async_trait::async_trait;
use std::collections::HashMap;

use crate::nation::Nation;

#[async_trait]
pub trait SendNations {
    async fn send_nations<F, O>(&self, function: F) -> O
    where
        O: Send + 'static,
        F: FnOnce(&mut HashMap<String, Nation>) -> O + Send + 'static;
}
