use super::*;

pub trait BuildQueue {
    fn queue(&mut self, build_instruction: BuildInstruction);
}
