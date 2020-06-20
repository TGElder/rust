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
        let (update_tx, update_rx) = update_channel(UPDATE_CHANNEL_BOUND);

        Simulation {
            processors,
            tx: update_tx,
            rx: update_rx,
            run: true,
            state: Some(State::default()),
        }
    }

    pub fn tx(&self) -> &UpdateSender<Simulation> {
        &self.tx
    }

    pub fn run(&mut self) {
        while self.run {
            self.process_updates();

            if !self.run {
                return;
            }

            self.process_instructions();
        }
    }

    fn process_updates(&mut self) {
        let updates = self.rx.get_updates();
        process_updates(updates, self);
    }

    fn process_instructions(&mut self) {
        let mut state = self.state.take().expect("Simulation has lost state!");
        state.instructions.push(Instruction::Step);
        while let Some(instruction) = state.instructions.pop() {
            for processor in self.processors.iter_mut() {
                state = processor.process(state, &instruction);
            }
        }
        self.state = Some(state);
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
                state.instructions = vec![Instruction::Town(v2(1, 1))];
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
            let mut runner = Simulation::new(vec![]);
            let tx = runner.tx().clone();
            let handle = thread::spawn(move || runner.run());
            block_on(async { tx.update(|sim| sim.shutdown()).await });
            handle.join().unwrap();
            done_2.store(true, Ordering::Relaxed);
        });

        let start = Instant::now();
        while !done.load(Ordering::Relaxed) {
            if start.elapsed().as_secs() > 10 {
                panic!("Simulation runner has not shutdown after 10 seconds");
            }
        }
    }

    #[test]
    fn should_add_step_to_instructions() {
        let processor = InstructionRetriever::new();
        let instructions = processor.instructions.clone();
        let mut runner = Simulation::new(vec![Box::new(processor)]);
        let tx = runner.tx().clone();

        let handle = thread::spawn(move || runner.run());
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
        let mut runner = Simulation::new(vec![Box::new(introducer), Box::new(receiver)]);
        let tx = runner.tx().clone();

        let handle = thread::spawn(move || runner.run());
        let start = Instant::now();
        while !instructions
            .lock()
            .unwrap()
            .contains(&Instruction::Town(v2(1, 1)))
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
        let mut runner = Simulation::new(vec![Box::new(processor_1), Box::new(processor_2)]);
        let tx = runner.tx().clone();

        let handle = thread::spawn(move || runner.run());
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
        let mut runner = Simulation::new(vec![Box::new(processor)]);
        let tx = runner.tx().clone();

        tx.update(|sim| sim.shutdown());
        let handle = thread::spawn(move || runner.run());
        handle.join().unwrap();

        assert!(instructions.lock().unwrap().is_empty());
    }

    #[test]
    fn save_load_round_trip() {
        let mut runner_1 = Simulation::new(vec![]);
        runner_1.state = Some(State {
            instructions: vec![
                Instruction::Town(v2(1, 1)),
                Instruction::Town(v2(2, 2)),
                Instruction::Town(v2(3, 3)),
            ],
        });
        runner_1.save("test_save");

        let mut runner_2 = Simulation::new(vec![]);
        runner_2.load("test_save");

        assert_eq!(runner_1.state, runner_2.state);
    }
}
