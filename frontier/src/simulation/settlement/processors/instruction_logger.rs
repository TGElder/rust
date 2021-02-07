use super::*;

use crate::settlement::Settlement;
use commons::log::debug;

pub struct InstructionLogger {}

#[async_trait]
#[allow(clippy::single_match)]
impl Processor for InstructionLogger {
    async fn process(&mut self, state: State, instruction: &Instruction) -> State {
        match instruction {
            Instruction::GetDemand(Settlement {
                name,
                current_population,
                ..
            }) => debug!("{} ({})", name, current_population),
            _ => (),
        };
        state
    }
}

impl InstructionLogger {
    pub fn new() -> InstructionLogger {
        InstructionLogger {}
    }
}
