use super::*;

use crate::settlement::Settlement;

pub struct InstructionLogger {}

impl Processor for InstructionLogger {
    fn process(&mut self, state: State, instruction: &Instruction) -> State {
        match instruction {
            Instruction::GetDemand(Settlement {
                name,
                current_population,
                ..
            }) => println!("{} ({})", name, current_population),
            Instruction::GetRoutes(Demand { resource, .. }) => println!("  - {}", resource.name()),
            Instruction::GetRouteChanges { route_set, .. } => {
                println!("    - found {} routes", route_set.len())
            }
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
