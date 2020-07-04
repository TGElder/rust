use super::*;

use crate::game::traits::Micros;
use commons::futures::executor::block_on;
use commons::update::*;
use std::collections::BinaryHeap;
use std::fs::File;
use std::io::{BufReader, BufWriter};

const HANDLE: &str = "build_service";
const UPDATE_CHANNEL_BOUND: usize = 100;

pub struct BuildService<G>
where
    G: Micros,
{
    game: UpdateSender<G>,
    builders: Vec<Box<dyn Builder + Send>>,
    tx: UpdateSender<BuildService<G>>,
    rx: UpdateReceiver<BuildService<G>>,
    queue: BinaryHeap<BuildInstruction>,
    run: bool,
}

impl<G> BuildService<G>
where
    G: Micros,
{
    pub fn new(game: &UpdateSender<G>, builders: Vec<Box<dyn Builder + Send>>) -> BuildService<G> {
        let (tx, rx) = update_channel(UPDATE_CHANNEL_BOUND);

        BuildService {
            game: game.clone_with_handle(HANDLE),
            builders,
            tx,
            rx,
            queue: BinaryHeap::new(),
            run: true,
        }
    }

    pub fn tx(&self) -> &UpdateSender<BuildService<G>> {
        &self.tx
    }

    pub fn queue(&mut self, build_instruction: BuildInstruction) {
        self.queue.push(build_instruction);
    }

    pub fn run(&mut self) {
        while self.run {
            self.process_updates();
            self.build_next();
        }
    }

    fn process_updates(&mut self) {
        let updates = self.rx.get_updates();
        process_updates(updates, self);
    }

    fn build_next(&mut self) {
        if let Some(build) = self.next() {
            self.build(build);
        }
    }

    fn next(&mut self) -> Option<Build> {
        let micros = self.micros();
        match self.queue.peek() {
            Some(BuildInstruction { when, .. }) if *when <= micros => (),
            _ => return None,
        };
        let BuildInstruction { what: build, .. } = self.queue.pop().unwrap();
        Some(build)
    }

    fn build(&mut self, build: Build) {
        for builder in self.builders.iter_mut() {
            if builder.can_build(&build) {
                builder.build(build);
                return;
            }
        }
    }

    fn micros(&mut self) -> u128 {
        block_on(async { self.game.update(|game| *game.micros()).await })
    }

    pub fn shutdown(&mut self) {
        self.run = false;
    }

    fn get_path(path: &str) -> String {
        format!("{}.build_service", path)
    }

    pub fn save(&mut self, path: &str) {
        let path = Self::get_path(&path);
        let mut file = BufWriter::new(File::create(path).unwrap());
        bincode::serialize_into(&mut file, &self.queue).unwrap();
    }

    pub fn load(&mut self, path: &str) {
        let path = Self::get_path(path);
        let file = BufReader::new(File::open(path).unwrap());
        self.queue = bincode::deserialize_from(file).unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use commons::update::{process_updates, update_channel};
    use commons::v2;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::thread::JoinHandle;
    use std::time::Instant;

    struct BuildRetriever {
        builds: Arc<Mutex<Vec<Build>>>,
    }

    impl BuildRetriever {
        fn new() -> BuildRetriever {
            BuildRetriever {
                builds: Arc::new(Mutex::new(vec![])),
            }
        }
    }

    impl Builder for BuildRetriever {
        fn can_build(&self, _: &Build) -> bool {
            true
        }

        fn build(&mut self, build: Build) {
            self.builds.lock().unwrap().push(build);
        }
    }

    fn game(mut micros: u128) -> (UpdateSender<u128>, JoinHandle<()>, Arc<Mutex<AtomicBool>>) {
        let (game, mut rx) = update_channel(100);
        let run = Arc::new(Mutex::new(AtomicBool::new(true)));
        let run_2 = run.clone();
        let game_handle = thread::spawn(move || {
            while run_2.lock().unwrap().load(Ordering::Relaxed) {
                let updates = rx.get_updates();
                process_updates(updates, &mut micros);
            }
        });
        (game, game_handle, run)
    }

    #[test]
    fn should_hand_build_to_builder_if_when_elapsed() {
        // Given
        let (game, game_handle, game_run) = game(1000);

        let retriever = BuildRetriever::new();
        let builds = retriever.builds.clone();

        let mut build_service = BuildService::new(&game, vec![Box::new(retriever)]);
        build_service.queue(BuildInstruction {
            what: Build::Road(v2(1, 2)),
            when: 100,
        });
        let tx = build_service.tx().clone();

        // When
        let build_service_handle = thread::spawn(move || build_service.run());
        let start = Instant::now();
        while builds.lock().unwrap().is_empty() && start.elapsed().as_secs() < 10 {}

        block_on(async { tx.update(|sim| sim.shutdown()).await });
        build_service_handle.join().unwrap();
        game_run.lock().unwrap().store(false, Ordering::Relaxed);
        game_handle.join().unwrap();

        // Then
        assert_eq!(*builds.lock().unwrap(), vec![Build::Road(v2(1, 2))]);
    }

    #[test]
    fn should_not_hand_build_to_builder_if_when_not_elapsed() {
        // Given
        let (game, game_handle, game_run) = game(1000);

        let retriever = BuildRetriever::new();
        let builds = retriever.builds.clone();

        let mut build_service = BuildService::new(&game, vec![Box::new(retriever)]);
        build_service.queue(BuildInstruction {
            what: Build::Road(v2(1, 2)),
            when: 100,
        });
        build_service.queue(BuildInstruction {
            what: Build::Road(v2(3, 4)),
            when: 2000,
        });
        let tx = build_service.tx().clone();

        // When
        let build_service_handle = thread::spawn(move || build_service.run());
        let start = Instant::now();
        while builds.lock().unwrap().is_empty() && start.elapsed().as_secs() < 10 {}

        block_on(async { tx.update(|sim| sim.shutdown()).await });
        build_service_handle.join().unwrap();
        game_run.lock().unwrap().store(false, Ordering::Relaxed);
        game_handle.join().unwrap();

        // Then
        assert_eq!(*builds.lock().unwrap(), vec![Build::Road(v2(1, 2))]);
    }

    #[test]
    fn should_order_builds_by_when() {
        // Given
        let (game, game_handle, game_run) = game(1000);

        let retriever = BuildRetriever::new();
        let builds = retriever.builds.clone();

        let mut build_service = BuildService::new(&game, vec![Box::new(retriever)]);
        build_service.queue(BuildInstruction {
            what: Build::Road(v2(1, 2)),
            when: 200,
        });
        build_service.queue(BuildInstruction {
            what: Build::Road(v2(3, 4)),
            when: 100,
        });
        let tx = build_service.tx().clone();

        // When
        let build_service_handle = thread::spawn(move || build_service.run());
        let start = Instant::now();
        while builds.lock().unwrap().is_empty() && start.elapsed().as_secs() < 10 {}

        block_on(async { tx.update(|sim| sim.shutdown()).await });
        build_service_handle.join().unwrap();
        game_run.lock().unwrap().store(false, Ordering::Relaxed);
        game_handle.join().unwrap();

        // Then
        assert_eq!(
            *builds.lock().unwrap(),
            vec![Build::Road(v2(3, 4)), Build::Road(v2(1, 2))]
        );
    }

    #[test]
    fn should_shutdown() {
        // Given
        let (game, game_handle, game_run) = game(1000);
        let mut build_service = BuildService::new(&game, vec![]);

        // When
        let done = Arc::new(AtomicBool::new(false));
        let done_2 = done.clone();
        thread::spawn(move || {
            let tx = build_service.tx().clone();
            let handle = thread::spawn(move || build_service.run());
            block_on(async { tx.update(|build_service| build_service.shutdown()).await });
            handle.join().unwrap();
            done_2.store(true, Ordering::Relaxed);
        });

        // Then
        let start = Instant::now();
        while !done.load(Ordering::Relaxed) {
            if start.elapsed().as_secs() > 10 {
                panic!("Build service has not shutdown after 10 seconds");
            }
        }

        // Finally
        game_run.lock().unwrap().store(false, Ordering::Relaxed);
        game_handle.join().unwrap();
    }

    #[test]
    fn save_load_round_trip() {
        // Given
        let (game, game_handle, game_run) = game(1000);
        let mut build_service_1 = BuildService::new(&game, vec![]);
        build_service_1.queue(BuildInstruction {
            what: Build::Road(v2(1, 2)),
            when: 200,
        });
        build_service_1.queue(BuildInstruction {
            what: Build::Road(v2(3, 4)),
            when: 100,
        });
        build_service_1.save("test_save");

        let mut build_service_2 = BuildService::new(&game, vec![]);

        // When
        build_service_2.load("test_save");

        // Then
        let queue_1: Vec<BuildInstruction> = build_service_1.queue.drain().collect();
        let queue_2: Vec<BuildInstruction> = build_service_2.queue.drain().collect();
        assert_eq!(queue_1, queue_2);

        game_run.lock().unwrap().store(false, Ordering::Relaxed);
        game_handle.join().unwrap();
    }
}
