use super::*;

pub trait Processor {
    fn process(&mut self, state: State, instruction: &Instruction) -> State;
}
