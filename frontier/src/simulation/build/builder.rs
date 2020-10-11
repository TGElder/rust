use super::*;

#[async_trait]
pub trait Builder {
    fn can_build(&self, build: &Build) -> bool;
    async fn build(&mut self, build: Build);
}
