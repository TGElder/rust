use super::*;

use commons::update::*;
use std::fs::File;
use std::io::{BufReader, BufWriter};

const UPDATE_CHANNEL_BOUND: usize = 100;

pub struct Simulation {
    processors: Vec<Box<dyn Processor + Send>>,
    state: Option<State>,
    tx: UpdateSender<Simulation>,
    rx: UpdateReceiver<Simulation>,
    run: bool,
}

impl Simulation {
    pub fn new(processors: Vec<Box<dyn Processor + Send>>) -> Simulation {
        let (tx, rx) = update_channel(UPDATE_CHANNEL_BOUND);

        Simulation {
            processors,
            tx,
            rx,
            run: true,
            state: None,
        }
    }

    pub fn tx(&self) -> &UpdateSender<Simulation> {
        &self.tx
    }

    pub fn set_state(&mut self, state: State) {
        self.state = Some(state);
    }

    pub fn run(&mut self) {
        while self.run {
            self.process_updates();

            if !self.run {
                return;
            }

            self.process_instructions();
            self.step();
        }
    }

    fn process_updates(&mut self) {
        let updates = self.rx.get_updates();
        process_updates(updates, self);
    }

    fn process_instructions(&mut self) {
        let mut state = unwrap_or!(self.state.take(), return);
        while let Some(instruction) = state.instructions.pop() {
            for processor in self.processors.iter_mut() {
                state = processor.process(state, &instruction);
            }
        }
        self.state = Some(state);
    }

    fn step(&mut self) {
        let state = unwrap_or!(self.state.as_mut(), return);
        state.instructions.push(Instruction::Step);
    }

    pub fn shutdown(&mut self) {
        self.run = false;
    }

    fn get_path(path: &str) -> String {
        format!("{}.sim", path)
    }

    pub fn save(&mut self, path: &str) {
        let path = Self::get_path(path);
        let mut file = BufWriter::new(File::create(path).unwrap());
        bincode::serialize_into(&mut file, &self.state).unwrap();
    }

    pub fn load(&mut self, path: &str) {
        let path = Self::get_path(path);
        let file = BufReader::new(File::open(path).unwrap());
        self.state = bincode::deserialize_from(file).unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::route::RouteKey;
    use crate::world::Resource;
    use commons::index2d::Vec2D;
    use commons::v2;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::Instant;

    struct InstructionRetriever {
        instructions: Arc<Mutex<Vec<Instruction>>>,
    }

    impl InstructionRetriever {
        fn new() -> InstructionRetriever {
            InstructionRetriever {
                instructions: Arc::new(Mutex::new(vec![])),
            }
        }
    }

    impl Processor for InstructionRetriever {
        fn process(&mut self, state: State, instruction: &Instruction) -> State {
            self.instructions.lock().unwrap().push(instruction.clone());
            state
        }
    }

    struct InstructionIntroducer {
        introduced: bool,
    }

    impl InstructionIntroducer {
        fn new() -> InstructionIntroducer {
            InstructionIntroducer { introduced: false }
        }
    }

    impl Processor for InstructionIntroducer {
        fn process(&mut self, mut state: State, _: &Instruction) -> State {
            if !self.introduced {
                state.instructions = vec![Instruction::SettlementRef(v2(1, 1))];
                self.introduced = true;
            }
            state
        }
    }

    #[test]
    fn should_shutdown() {
        let done = Arc::new(AtomicBool::new(false));
        let done_2 = done.clone();
        thread::spawn(move || {
            let mut sim = Simulation::new(vec![]);
            sim.set_state(State::default());
            let tx = sim.tx().clone();
            let handle = thread::spawn(move || sim.run());
            block_on(async { tx.update(|sim| sim.shutdown()).await });
            handle.join().unwrap();
            done_2.store(true, Ordering::Relaxed);
        });

        let start = Instant::now();
        while !done.load(Ordering::Relaxed) {
            if start.elapsed().as_secs() > 10 {
                panic!("Simulation has not shutdown after 10 seconds");
            }
        }
    }

    #[test]
    fn should_add_step_to_instructions() {
        let processor = InstructionRetriever::new();
        let instructions = processor.instructions.clone();
        let mut sim = Simulation::new(vec![Box::new(processor)]);
        sim.set_state(State::default());
        let tx = sim.tx().clone();

        let handle = thread::spawn(move || sim.run());
        let start = Instant::now();
        while !instructions.lock().unwrap().contains(&Instruction::Step) {
            if start.elapsed().as_secs() > 10 {
                panic!("No step instruction received after 10 seconds");
            }
        }
        block_on(async { tx.update(|sim| sim.shutdown()).await });
        handle.join().unwrap();
    }

    #[test]
    fn should_update_state() {
        let introducer = InstructionIntroducer::new();
        let receiver = InstructionRetriever::new();
        let instructions = receiver.instructions.clone();
        let mut sim = Simulation::new(vec![Box::new(introducer), Box::new(receiver)]);
        sim.set_state(State::default());
        let tx = sim.tx().clone();

        let handle = thread::spawn(move || sim.run());
        let start = Instant::now();
        while !instructions
            .lock()
            .unwrap()
            .contains(&Instruction::SettlementRef(v2(1, 1)))
        {
            if start.elapsed().as_secs() > 10 {
                panic!("No town instruction received after 10 seconds");
            }
        }
        block_on(async { tx.update(|sim| sim.shutdown()).await });
        handle.join().unwrap();
    }

    #[test]
    fn should_hand_instructions_to_all_processors() {
        let processor_1 = InstructionRetriever::new();
        let instructions_1 = processor_1.instructions.clone();
        let processor_2 = InstructionRetriever::new();
        let instructions_2 = processor_2.instructions.clone();
        let mut sim = Simulation::new(vec![Box::new(processor_1), Box::new(processor_2)]);
        sim.set_state(State::default());
        let tx = sim.tx().clone();

        let handle = thread::spawn(move || sim.run());
        let start = Instant::now();
        while start.elapsed().as_secs() < 10
            && (instructions_1.lock().unwrap().is_empty()
                || instructions_2.lock().unwrap().is_empty())
        {}
        block_on(async { tx.update(|sim| sim.shutdown()).await });
        handle.join().unwrap();

        assert_eq!(instructions_1.lock().unwrap()[0], Instruction::Step);
        assert_eq!(instructions_2.lock().unwrap()[0], Instruction::Step);
    }

    #[test]
    fn should_process_updates_before_instructions() {
        let processor = InstructionRetriever::new();
        let instructions = processor.instructions.clone();
        let mut sim = Simulation::new(vec![Box::new(processor)]);
        sim.set_state(State::default());
        let tx = sim.tx().clone();

        tx.update(|sim| sim.shutdown());
        let handle = thread::spawn(move || sim.run());
        handle.join().unwrap();

        assert!(instructions.lock().unwrap().is_empty());
    }

    #[test]
    fn save_load_round_trip() {
        let mut sim_1 = Simulation::new(vec![]);
        let route_key = RouteKey {
            settlement: v2(1, 2),
            resource: Resource::Crabs,
            destination: v2(3, 4),
        };
        sim_1.set_state(State {
            instructions: vec![
                Instruction::SettlementRef(v2(1, 1)),
                Instruction::SettlementRef(v2(2, 2)),
                Instruction::SettlementRef(v2(3, 3)),
            ],
            traffic: Vec2D::new(3, 5, [route_key].iter().cloned().collect()),
        });
        sim_1.save("test_save");

        let mut sim_2 = Simulation::new(vec![]);
        sim_2.load("test_save");

        assert_eq!(sim_1.state, sim_2.state);
    }

    #[test]
    fn should_not_step_after_loading_instructions() {
        let mut sim_1 = Simulation::new(vec![]);
        sim_1.set_state(State {
            instructions: vec![Instruction::SettlementRef(v2(1, 1))],
            ..State::default()
        });
        sim_1.save("test_save");

        let receiver = InstructionRetriever::new();
        let instructions = receiver.instructions.clone();

        let mut sim_2 = Simulation::new(vec![Box::new(receiver)]);
        sim_2.load("test_save");
        let tx = sim_2.tx().clone();

        let handle = thread::spawn(move || sim_2.run());
        let start = Instant::now();
        while !instructions
            .lock()
            .unwrap()
            .contains(&Instruction::SettlementRef(v2(1, 1)))
        {
            if start.elapsed().as_secs() > 10 {
                panic!("No town instruction received after 10 seconds");
            }
        }
        block_on(async { tx.update(|sim| sim.shutdown()).await });
        handle.join().unwrap();

        assert_eq!(
            instructions.lock().unwrap()[0],
            Instruction::SettlementRef(v2(1, 1))
        );
    }
}
