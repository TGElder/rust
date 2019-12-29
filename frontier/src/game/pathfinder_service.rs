use super::*;
use crate::pathfinder::*;
use crate::travel_duration::*;
use std::sync::{Arc, RwLock};
use std::thread;
use std::thread::JoinHandle;

pub enum PathfinderCommand<T>
where
    T: TravelDuration,
{
    Use(Box<dyn FnOnce(&Pathfinder<T>) -> Vec<GameCommand> + Send>),
    Update(Box<dyn FnOnce(&mut Pathfinder<T>) + Send>),
    Shutdown,
}

struct Service<T>
where
    T: TravelDuration,
{
    game_command_tx: Sender<GameCommand>,
    command_rx: Receiver<PathfinderCommand<T>>,
    pathfinder: Arc<RwLock<Pathfinder<T>>>,
}

impl<T> Service<T>
where
    T: TravelDuration,
{
    fn execute(&self, function: Box<dyn FnOnce(&Pathfinder<T>) -> Vec<GameCommand>>) {
        let commands = function(&self.pathfinder.read().unwrap());
        for command in commands {
            self.game_command_tx.send(command).unwrap();
        }
    }

    fn update(&mut self, function: Box<dyn FnOnce(&mut Pathfinder<T>)>) {
        function(&mut self.pathfinder.write().unwrap());
    }

    fn run(&mut self) {
        loop {
            match self.command_rx.recv().unwrap() {
                PathfinderCommand::Use(function) => self.execute(function),
                PathfinderCommand::Update(function) => self.update(function),
                PathfinderCommand::Shutdown => return,
            }
        }
    }
}

pub struct PathfinderServiceEventConsumer<T>
where
    T: TravelDuration,
{
    command_tx: Sender<PathfinderCommand<T>>,
    pathfinder: Arc<RwLock<Pathfinder<T>>>,
    join_handle: Option<JoinHandle<()>>,
}

impl<T> PathfinderServiceEventConsumer<T>
where
    T: TravelDuration + Sync + 'static,
{
    pub fn new(
        game_command_tx: Sender<GameCommand>,
        pathfinder: Pathfinder<T>,
    ) -> PathfinderServiceEventConsumer<T> {
        let pathfinder = RwLock::new(pathfinder);
        let pathfinder = Arc::new(pathfinder);

        let (command_tx, command_rx) = mpsc::channel();

        let mut service = Service {
            game_command_tx,
            command_rx,
            pathfinder: pathfinder.clone(),
        };

        let join_handle = thread::spawn(move || {
            service.run();
        });

        PathfinderServiceEventConsumer {
            command_tx,
            pathfinder,
            join_handle: Some(join_handle),
        }
    }

    pub fn command_tx(&self) -> Sender<PathfinderCommand<T>> {
        self.command_tx.clone()
    }

    fn reset_pathfinder(&mut self, game_state: &GameState) {
        self.pathfinder
            .write()
            .unwrap()
            .reset_edges(&game_state.world);
    }

    fn update_pathfinder_with_cells(&mut self, game_state: &GameState, cells: &[V2<usize>]) {
        for cell in cells {
            self.pathfinder
                .write()
                .unwrap()
                .update_node(&game_state.world, cell);
        }
    }

    fn update_pathfinder_with_roads(&mut self, game_state: &GameState, result: &RoadBuilderResult) {
        result.update_pathfinder(&game_state.world, &mut self.pathfinder.write().unwrap());
    }

    fn shutdown(&mut self) {
        self.command_tx.send(PathfinderCommand::Shutdown).unwrap();
        if let Some(join_handle) = self.join_handle.take() {
            join_handle.join().unwrap();
        }
    }
}

impl<T> GameEventConsumer for PathfinderServiceEventConsumer<T>
where
    T: TravelDuration + Sync + 'static,
{
    fn consume_game_event(&mut self, game_state: &GameState, event: &GameEvent) -> CaptureEvent {
        match event {
            GameEvent::CellsRevealed(selection) => {
                match selection {
                    CellSelection::All => self.reset_pathfinder(game_state),
                    CellSelection::Some(cells) => {
                        self.update_pathfinder_with_cells(game_state, &cells)
                    }
                };
            }
            GameEvent::RoadsUpdated(result) => {
                self.update_pathfinder_with_roads(game_state, result)
            }
            _ => (),
        }
        CaptureEvent::No
    }

    fn consume_engine_event(&mut self, _: &GameState, event: Arc<Event>) -> CaptureEvent {
        if let Event::Shutdown = *event {
            self.shutdown();
        }
        CaptureEvent::No
    }
}
