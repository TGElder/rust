use commons::async_trait::async_trait;

use crate::simulation::{BuildInstruction, BuildKey};
use crate::traits::SendBuildQueue;

#[async_trait]
pub trait InsertBuildInstruction {
    async fn insert_build_instruction(&self, build_instruction: BuildInstruction);
}

#[async_trait]
impl<T> InsertBuildInstruction for T
where
    T: SendBuildQueue + Send + Sync,
{
    async fn insert_build_instruction(&self, build_instruction: BuildInstruction) {
        self.mut_build_queue(move |queue| queue.insert(build_instruction))
            .await;
    }
}

#[async_trait]
pub trait RemoveBuildInstruction {
    async fn remove_build_instruction(&self, build_key: &BuildKey);
}

#[async_trait]
impl<T> RemoveBuildInstruction for T
where
    T: SendBuildQueue + Send + Sync,
{
    async fn remove_build_instruction(&self, build_key: &BuildKey) {
        self.mut_build_queue(move |queue| queue.remove(build_key))
            .await;
    }
}

#[async_trait]
pub trait TakeBuildInstructionsBefore {
    async fn take_build_instructions_before(&self, micros: &u128) -> Vec<BuildInstruction>;
}

#[async_trait]
impl<T> TakeBuildInstructionsBefore for T
where
    T: SendBuildQueue + Send + Sync,
{
    async fn take_build_instructions_before(&self, micros: &u128) -> Vec<BuildInstruction> {
        self.mut_build_queue(move |queue| queue.take_instructions_before(micros))
            .await
    }
}

#[async_trait]
pub trait GetBuildInstruction {
    async fn get_build_instruction(&self, build_key: &BuildKey) -> Option<BuildInstruction>;
}

#[async_trait]
impl<T> GetBuildInstruction for T
where
    T: SendBuildQueue + Send + Sync,
{
    async fn get_build_instruction(&self, build_key: &BuildKey) -> Option<BuildInstruction> {
        self.get_build_queue(|queue| queue.get(build_key).cloned())
            .await
    }
}
