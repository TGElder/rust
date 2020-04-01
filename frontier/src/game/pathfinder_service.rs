use super::*;
use crate::pathfinder::*;
use crate::travel_duration::*;
use commons::Arm;
use std::sync::MutexGuard;

const UPDATE_CHANNEL_BOUND: usize = 10_000;

pub struct PathfinderService<T>
where
    T: TravelDuration,
{
    update_tx: UpdateSender<PathfinderService<T>>,
    update_rx: UpdateReceiver<PathfinderService<T>>,
    pathfinder: Arm<Pathfinder<T>>,
    run: bool,
}

impl<T> PathfinderService<T>
where
    T: TravelDuration,
{
    pub fn new(pathfinder: Arm<Pathfinder<T>>) -> PathfinderService<T> {
        let (update_tx, update_rx) = update_channel(UPDATE_CHANNEL_BOUND);
        PathfinderService {
            update_tx,
            update_rx,
            pathfinder,
            run: true,
        }
    }

    pub fn update_tx(&self) -> &UpdateSender<PathfinderService<T>> {
        &self.update_tx
    }

    pub fn pathfinder(&mut self) -> MutexGuard<Pathfinder<T>> {
        self.pathfinder.lock().unwrap()
    }

    pub fn shutdown(&mut self) {
        self.run = false;
    }

    pub fn run(&mut self) {
        while self.run {
            let updates = self.update_rx.get_updates();
            process_updates(updates, self);
        }
    }
}
