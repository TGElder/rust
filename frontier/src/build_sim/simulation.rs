use commons::bincode::{deserialize_from, serialize_into};
use commons::edge::Edge;
use commons::process::Step;

use super::*;

use std::collections::HashSet;
use std::fs::File;
use std::io::{BufReader, BufWriter};

pub struct BuildSimulation {
    processors: Vec<Box<dyn Processor + Send>>,
    state: Option<State>,
}

impl BuildSimulation {
    pub fn new(processors: Vec<Box<dyn Processor + Send>>) -> BuildSimulation {
        BuildSimulation {
            processors,
            state: None,
        }
    }

    pub async fn new_game(&mut self) {
        self.state = Some(State {
            instructions: vec![],
        });
    }

    pub fn refresh_edges(&mut self, edges: HashSet<Edge>) {
        if let Some(state) = &mut self.state {
            state.instructions.push(Instruction::RefreshEdges(edges));
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
impl Step for BuildSimulation {
    async fn step(&mut self) {
        let state = unwrap_or!(self.state.take(), return);
        let state = self.process_instruction(state).await;
        self.state = Some(state);
    }
}
