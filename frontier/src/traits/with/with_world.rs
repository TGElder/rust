use crate::world::World;
use commons::async_trait::async_trait;

#[async_trait]
pub trait WithWorld {
    async fn with_world<F, O>(&self, function: F) -> O
    where
        F: FnOnce(&World) -> O + Send;

    async fn mut_world<F, O>(&self, function: F) -> O
    where
        F: FnOnce(&mut World) -> O + Send;
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;
    #[async_trait]
    impl WithWorld for Mutex<World> {
        async fn with_world<F, O>(&self, function: F) -> O
        where
            F: FnOnce(&World) -> O + Send,
        {
            function(&self.lock().unwrap())
        }

        async fn mut_world<F, O>(&self, function: F) -> O
        where
            F: FnOnce(&mut World) -> O + Send,
        {
            function(&mut self.lock().unwrap())
        }
    }
}
