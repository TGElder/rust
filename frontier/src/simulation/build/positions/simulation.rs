use commons::process::Step;

use super::*;

use std::collections::HashSet;

pub struct PositionBuildSimulation {
    processors: Vec<Box<dyn Processor + Send>>,
    state: Option<State>,
}

impl PositionBuildSimulation {
    pub fn new(processors: Vec<Box<dyn Processor + Send>>) -> PositionBuildSimulation {
        PositionBuildSimulation {
            processors,
            state: Some(State {
                instructions: vec![],
            }),
        }
    }

    pub fn refresh_positions(&mut self, positions: HashSet<V2<usize>>) {
        if let Some(state) = &mut self.state {
            state
                .instructions
                .push(Instruction::RefreshPositions(positions));
        }
    }

    async fn process_instruction(&mut self, mut state: State) -> State {
        if let Some(instruction) = state.instructions.pop() {
            for processor in self.processors.iter_mut() {
                state = processor.process(state, &instruction).await;
            }
        }
        state
    }
}

#[async_trait]
impl Step for PositionBuildSimulation {
    async fn step(&mut self) {
        let state = unwrap_or!(self.state.take(), return);
        let state = self.process_instruction(state).await;
        self.state = Some(state);
    }
}
