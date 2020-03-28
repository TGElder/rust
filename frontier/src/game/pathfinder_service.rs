use super::*;
use crate::pathfinder::*;
use crate::travel_duration::*;
use commons::Arm;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::thread;

const UPDATE_CHANNEL_BOUND: usize = 10_000;
const LOAD_ORDERING: Ordering = Ordering::Relaxed;
const STORE_ORDERING: Ordering = Ordering::Relaxed;

struct Service<T>
where
    T: TravelDuration,
{
    update_rx: UpdateReceiver<Pathfinder<T>>,
    pathfinder: Arm<Pathfinder<T>>,
    run: Arc<AtomicBool>,
    done: Arc<AtomicBool>,
}

impl<T> Service<T>
where
    T: TravelDuration,
{
    fn run(&mut self) {
        while self.run.load(LOAD_ORDERING) {
            let updates = self.update_rx.get_updates();
            process_updates(updates, &mut self.pathfinder.lock().unwrap());
        }
        self.done.store(true, STORE_ORDERING);
    }
}

pub struct PathfinderServiceEventConsumer<T>
where
    T: TravelDuration,
{
    update_tx: UpdateSender<Pathfinder<T>>,
    pathfinder: Arm<Pathfinder<T>>,
    run: Arc<AtomicBool>,
    done: Arc<AtomicBool>,
}

impl<T> PathfinderServiceEventConsumer<T>
where
    T: TravelDuration + Sync + 'static,
{
    pub fn new(pathfinder: Pathfinder<T>) -> PathfinderServiceEventConsumer<T> {
        let pathfinder = Arc::new(Mutex::new(pathfinder));

        let (update_tx, update_rx) = update_channel(UPDATE_CHANNEL_BOUND);

        let run = Arc::new(AtomicBool::new(true));
        let done = Arc::new(AtomicBool::new(false));

        let mut service = Service {
            update_rx,
            pathfinder: pathfinder.clone(),
            run: run.clone(),
            done: done.clone(),
        };

        thread::spawn(move || {
            service.run();
        });

        PathfinderServiceEventConsumer {
            update_tx,
            pathfinder,
            run,
            done,
        }
    }

    pub fn update_tx(&self) -> &UpdateSender<Pathfinder<T>> {
        &self.update_tx
    }

    fn reset_pathfinder(&mut self, game_state: &GameState) {
        self.pathfinder
            .lock()
            .unwrap()
            .reset_edges(&game_state.world);
    }

    fn update_pathfinder_with_cells(&mut self, game_state: &GameState, cells: &[V2<usize>]) {
        for cell in cells {
            self.pathfinder
                .lock()
                .unwrap()
                .update_node(&game_state.world, cell);
        }
    }

    fn update_pathfinder_with_roads(&mut self, game_state: &GameState, result: &RoadBuilderResult) {
        result.update_pathfinder(&game_state.world, &mut self.pathfinder.lock().unwrap());
    }
}

impl<T> GameEventConsumer for PathfinderServiceEventConsumer<T>
where
    T: TravelDuration + Sync + 'static,
{
    fn name(&self) -> &'static str {
        "pathfinder_service_event_consumer"
    }

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

    fn consume_engine_event(&mut self, _: &GameState, _: Arc<Event>) -> CaptureEvent {
        CaptureEvent::No
    }

    fn shutdown(&mut self) {
        self.run.store(false, STORE_ORDERING);
    }

    fn is_shutdown(&self) -> bool {
        self.done.load(LOAD_ORDERING)
    }
}
