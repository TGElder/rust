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
    process_instructions: bool,
    run: bool,
}

impl Simulation {
    pub fn new(processors: Vec<Box<dyn Processor + Send>>) -> Simulation {
        let (tx, rx) = update_channel(UPDATE_CHANNEL_BOUND);

        Simulation {
            processors,
            tx,
            rx,
            process_instructions: false,
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

    pub fn start_processing_instructions(&mut self) {
        self.process_instructions = true;
    }

    pub fn stop_processing_instructions(&mut self) {
        self.process_instructions = false;
    }

    pub fn run(&mut self) {
        while self.run {
            self.process_updates();

            if !self.run {
                return;
            }

            if self.process_instructions {
                self.process_instruction();
                self.try_step();
            }
        }
    }

    fn process_updates(&mut self) {
        let updates = self.rx.get_updates();
        process_updates(updates, self);
    }

    fn process_instruction(&mut self) {
        let mut state = unwrap_or!(self.state.take(), return);
        if let Some(instruction) = state.instructions.pop() {
            for processor in self.processors.iter_mut() {
                state = processor.process(state, &instruction);
            }
        }
        self.state = Some(state);
    }

    fn try_step(&mut self) {
        let state = unwrap_or!(self.state.as_mut(), return);
        if state.instructions.is_empty() {
            state.instructions.push(Instruction::Step);
        }
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

    use crate::resource::Resource;
    use crate::route::RouteKey;
    use commons::edge::Edge;
    use commons::index2d::Vec2D;
    use commons::v2;
    use std::fs::remove_file;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::{Duration, Instant};

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

    #[test]
    fn should_shutdown() {
        let done = Arc::new(AtomicBool::new(false));
        let done_2 = done.clone();
        thread::spawn(move || {
            let mut sim = Simulation::new(vec![]);
            sim.start_processing_instructions();
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
        sim.start_processing_instructions();
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
                state.instructions = vec![Instruction::GetTerritory(v2(1, 1))];
                self.introduced = true;
            }
            state
        }
    }

    #[test]
    fn should_update_state() {
        let introducer = InstructionIntroducer::new();
        let receiver = InstructionRetriever::new();
        let instructions = receiver.instructions.clone();
        let mut sim = Simulation::new(vec![Box::new(introducer), Box::new(receiver)]);
        sim.start_processing_instructions();
        sim.set_state(State::default());
        let tx = sim.tx().clone();

        let handle = thread::spawn(move || sim.run());
        let start = Instant::now();
        while !instructions
            .lock()
            .unwrap()
            .contains(&Instruction::GetTerritory(v2(1, 1)))
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
        sim.start_processing_instructions();
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
        sim.start_processing_instructions();
        sim.set_state(State::default());
        let tx = sim.tx().clone();

        tx.update(|sim| sim.shutdown());
        let handle = thread::spawn(move || sim.run());
        handle.join().unwrap();

        assert!(instructions.lock().unwrap().is_empty());
    }

    #[test]
    fn save_load_round_trip() {
        // Given
        let file_name = "test_save.simulation.round_trip";

        let mut sim_1 = Simulation::new(vec![]);
        let route_key = RouteKey {
            settlement: v2(1, 2),
            resource: Resource::Crabs,
            destination: v2(3, 4),
        };
        let mut build_queue = BuildQueue::default();
        build_queue.insert(BuildInstruction {
            when: 808,
            what: Build::Road(Edge::new(v2(1, 2), v2(1, 3))),
        });
        sim_1.set_state(State {
            params: SimulationParams {
                traffic_to_population: 0.123,
                nation_flip_traffic_pc: 0.456,
                initial_town_population: 0.234,
                town_removal_population: 0.789,
            },
            instructions: vec![
                Instruction::GetTerritory(v2(1, 1)),
                Instruction::GetTerritory(v2(2, 2)),
                Instruction::GetTerritory(v2(3, 3)),
            ],
            traffic: Vec2D::new(3, 5, [route_key].iter().cloned().collect()),
            edge_traffic: hashmap! { Edge::new(v2(1, 2), v2(1, 3)) => hashset!{route_key} },
            route_to_ports: hashmap! { route_key => hashset!{ v2(1, 2), v2(3, 4) } },
            build_queue,
        });
        sim_1.save(file_name);

        // When
        let mut sim_2 = Simulation::new(vec![]);
        sim_2.load(file_name);

        // Then
        assert_eq!(sim_1.state, sim_2.state);

        // Finally
        remove_file(format!("{}.sim", file_name)).unwrap();
    }

    #[test]
    fn should_not_step_after_loading_instructions() {
        // Given
        let file_name = "test_save.simulation.should_not_step";

        let mut sim_1 = Simulation::new(vec![]);
        sim_1.set_state(State {
            instructions: vec![Instruction::GetTerritory(v2(1, 1))],
            ..State::default()
        });
        sim_1.save(file_name);

        let receiver = InstructionRetriever::new();
        let instructions = receiver.instructions.clone();

        // When
        let mut sim_2 = Simulation::new(vec![Box::new(receiver)]);
        sim_2.start_processing_instructions();
        sim_2.load(file_name);
        let tx = sim_2.tx().clone();

        let handle = thread::spawn(move || sim_2.run());
        let start = Instant::now();
        while !instructions
            .lock()
            .unwrap()
            .contains(&Instruction::GetTerritory(v2(1, 1)))
        {
            if start.elapsed().as_secs() > 10 {
                panic!("No town instruction received after 10 seconds");
            }
        }
        block_on(async { tx.update(|sim| sim.shutdown()).await });
        handle.join().unwrap();

        // Then
        assert_eq!(
            instructions.lock().unwrap()[0],
            Instruction::GetTerritory(v2(1, 1))
        );

        // Finally
        remove_file(format!("{}.sim", file_name)).unwrap();
    }

    struct InstructionRepeater {}

    impl InstructionRepeater {
        fn new() -> InstructionRepeater {
            InstructionRepeater {}
        }
    }

    impl Processor for InstructionRepeater {
        fn process(&mut self, mut state: State, instruction: &Instruction) -> State {
            thread::sleep(Duration::from_millis(10));
            state.instructions.push(instruction.clone());
            state
        }
    }

    #[test]
    fn should_shutdown_without_processing_pending_instructions() {
        let done = Arc::new(AtomicBool::new(false));
        let done_2 = done.clone();
        thread::spawn(move || {
            let repeater = InstructionRepeater::new();
            let receiver = InstructionRetriever::new();
            let instructions = receiver.instructions.clone();
            let mut sim = Simulation::new(vec![Box::new(repeater), Box::new(receiver)]);
            sim.start_processing_instructions();
            sim.set_state(State::default());

            let tx = sim.tx().clone();
            let start = Instant::now();
            let handle = thread::spawn(move || sim.run());
            while instructions.lock().unwrap().is_empty() {
                if start.elapsed().as_secs() > 10 {
                    panic!("Instruction not received after 10 seconds");
                }
            }

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
    fn should_not_process_instruction_if_start_processing_instructions_is_not_called() {
        // Given
        let mut sim = Simulation::new(vec![]);
        sim.set_state(State {
            instructions: vec![Instruction::GetTerritory(v2(1, 1))],
            ..State::default()
        });
        let tx = sim.tx().clone();
        let handle = thread::spawn(move || {
            sim.run();
            sim
        });

        // When
        block_on(async { tx.update(|_| {}).await });
        block_on(async { tx.update(|sim| sim.shutdown()).await });

        // Then
        let sim = handle.join().unwrap();
        assert_eq!(
            sim.state.unwrap().instructions,
            vec![Instruction::GetTerritory(v2(1, 1))]
        );
    }

    #[test]
    fn should_not_process_instruction_after_stop_processing_instructions_is_called() {
        // Given
        let mut sim = Simulation::new(vec![]);
        sim.start_processing_instructions();
        sim.stop_processing_instructions();
        sim.set_state(State {
            instructions: vec![Instruction::GetTerritory(v2(1, 1))],
            ..State::default()
        });
        let tx = sim.tx().clone();
        let handle = thread::spawn(move || {
            sim.run();
            sim
        });

        // When
        block_on(async { tx.update(|_| {}).await });
        block_on(async { tx.update(|sim| sim.shutdown()).await });

        // Then
        let sim = handle.join().unwrap();
        assert_eq!(
            sim.state.unwrap().instructions,
            vec![Instruction::GetTerritory(v2(1, 1))]
        );
    }
}
