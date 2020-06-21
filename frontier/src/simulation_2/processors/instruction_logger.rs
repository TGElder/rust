use super::*;

pub struct InstructionLogger {}

impl Processor for InstructionLogger {
    fn process(&mut self, state: State, instruction: &Instruction) -> State {
        println!("{:?}", instruction);
        state
    }
}

impl InstructionLogger {
    pub fn new() -> InstructionLogger {
        InstructionLogger {}
    }
}
