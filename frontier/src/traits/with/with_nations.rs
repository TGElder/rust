use std::collections::HashMap;

use commons::async_trait::async_trait;

use crate::nation::Nation;

#[async_trait]
pub trait WithNations {
    async fn with_nations<F, O>(&self, function: F) -> O
    where
        F: FnOnce(&HashMap<String, Nation>) -> O + Send;

    async fn mut_nations<F, O>(&self, function: F) -> O
    where
        F: FnOnce(&mut HashMap<String, Nation>) -> O + Send;
}
