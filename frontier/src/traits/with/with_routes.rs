use commons::async_trait::async_trait;

use crate::route::Routes;

#[async_trait]
pub trait WithRoutes {
    async fn with_routes<F, O>(&self, function: F) -> O
    where
        F: FnOnce(&Routes) -> O + Send;

    async fn mut_routes<F, O>(&self, function: F) -> O
    where
        F: FnOnce(&mut Routes) -> O + Send;
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    #[async_trait]
    impl WithRoutes for Mutex<Routes> {
        async fn with_routes<F, O>(&self, function: F) -> O
        where
            F: FnOnce(&Routes) -> O + Send,
        {
            function(&self.lock().unwrap())
        }

        async fn mut_routes<F, O>(&self, function: F) -> O
        where
            F: FnOnce(&mut Routes) -> O + Send,
        {
            function(&mut self.lock().unwrap())
        }
    }
}
