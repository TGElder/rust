use commons::bincode::{deserialize_from, serialize_into};
use commons::process::Step;

use super::*;

use std::fs::File;
use std::io::{BufReader, BufWriter};

pub struct SettlementSimulation {
    processors: Vec<Box<dyn Processor + Send>>,
    state: Option<State>,
}

impl SettlementSimulation {
    pub fn new(processors: Vec<Box<dyn Processor + Send>>) -> SettlementSimulation {
        SettlementSimulation {
            processors,
            state: None,
        }
    }

    pub async fn new_game(&mut self) {
        self.state = Some(State {
            instructions: vec![],
        });
    }

    async fn process_instruction(&mut self, mut state: State) -> State {
        if let Some(instruction) = state.instructions.pop() {
            for processor in self.processors.iter_mut() {
                state = processor.process(state, &instruction).await;
            }
        }
        state
    }

    fn try_step(&mut self, state: &mut State) {
        if state.instructions.is_empty() {
            state.instructions.push(Instruction::Step);
        }
    }

    pub fn save(&self, path: &str) {
        let path = get_path(path);
        let mut file = BufWriter::new(File::create(path).unwrap());
        serialize_into(&mut file, &self.state).unwrap();
    }

    pub fn load(&mut self, path: &str) {
        let path = get_path(path);
        let file = BufReader::new(File::open(path).unwrap());
        self.state = deserialize_from(file).unwrap();
    }
}

fn get_path(path: &str) -> String {
    format!("{}.sim", path)
}

#[async_trait]
impl Step for SettlementSimulation {
    async fn step(&mut self) {
        let state = unwrap_or!(self.state.take(), return);
        let mut state = self.process_instruction(state).await;
        self.try_step(&mut state);
        self.state = Some(state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use commons::{v2, Arm};
    use futures::executor::block_on;
    use std::fs::remove_file;

    #[test]
    fn should_hand_instructions_to_all_processors() {
        // Given
        struct InstructionRetriever {
            instructions: Arm<Vec<Instruction>>,
        }

        #[async_trait]
        impl Processor for InstructionRetriever {
            async fn process(&mut self, state: State, instruction: &Instruction) -> State {
                self.instructions.lock().unwrap().push(instruction.clone());
                state
            }
        }

        let instructions_1 = Arm::default();
        let instructions_2 = Arm::default();
        let retriever_1 = InstructionRetriever {
            instructions: instructions_1.clone(),
        };
        let retriever_2 = InstructionRetriever {
            instructions: instructions_2.clone(),
        };
        let mut sim = SettlementSimulation::new(vec![Box::new(retriever_1), Box::new(retriever_2)]);
        sim.state = Some(State {
            instructions: vec![Instruction::Step],
        });

        // When
        block_on(sim.step());

        // Then
        assert_eq!(*instructions_1.lock().unwrap(), vec![Instruction::Step]);
        assert_eq!(*instructions_2.lock().unwrap(), vec![Instruction::Step]);
    }

    #[test]
    fn should_add_step_instruction_if_instructions_are_empty() {
        // Given
        let mut sim = SettlementSimulation::new(vec![]);
        sim.state = Some(State::default());

        // When
        block_on(sim.step());

        // Then
        assert_eq!(sim.state.unwrap().instructions, vec![Instruction::Step]);
    }

    #[test]
    fn processors_should_be_able_to_update_state() {
        // Given
        struct InstructionIntroducer {}

        #[async_trait]
        impl Processor for InstructionIntroducer {
            async fn process(&mut self, mut state: State, _: &Instruction) -> State {
                state
                    .instructions
                    .push(Instruction::UpdateHomelandPopulation);
                state
            }
        }

        let mut sim = SettlementSimulation::new(vec![Box::new(InstructionIntroducer {})]);
        sim.state = Some(State {
            instructions: vec![Instruction::Step],
        });

        // When
        block_on(sim.step());

        // Then
        assert_eq!(
            sim.state.unwrap().instructions,
            vec![Instruction::UpdateHomelandPopulation]
        );
    }

    #[test]
    fn save_load_round_trip() {
        // Given
        let file_name = "test_save.simulation.round_trip";

        let mut sim_1 = SettlementSimulation::new(vec![]);
        sim_1.state = Some(State {
            instructions: vec![
                Instruction::GetTerritory(v2(1, 1)),
                Instruction::GetTerritory(v2(2, 2)),
                Instruction::GetTerritory(v2(3, 3)),
            ],
        });
        sim_1.save(file_name);

        // When
        let mut sim_2 = SettlementSimulation::new(vec![]);
        sim_2.load(file_name);

        // Then
        assert_eq!(sim_1.state, sim_2.state);

        // Finally
        remove_file(format!("{}.sim", file_name)).unwrap();
    }
}