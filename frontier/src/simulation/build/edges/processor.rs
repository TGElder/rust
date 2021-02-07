use super::*;

#[async_trait]
pub trait Processor {
    async fn process(&mut self, state: State, instruction: &Instruction) -> State;
}
