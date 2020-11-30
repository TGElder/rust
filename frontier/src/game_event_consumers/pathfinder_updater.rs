use super::*;
use crate::pathfinder::*;
use crate::travel_duration::*;
use std::sync::{Arc, RwLock, RwLockWriteGuard};

pub struct PathfinderUpdater<T>
where
    T: TravelDuration,
{
    pathfinder: Arc<RwLock<Pathfinder<T>>>,
}

impl<T> PathfinderUpdater<T>
where
    T: TravelDuration + Sync + 'static,
{
    pub fn new(pathfinder: &Arc<RwLock<Pathfinder<T>>>) -> PathfinderUpdater<T> {
        PathfinderUpdater {
            pathfinder: pathfinder.clone(),
        }
    }

    fn pathfinder(&mut self) -> RwLockWriteGuard<Pathfinder<T>> {
        self.pathfinder.write().unwrap()
    }

    fn reset_pathfinder(&mut self, game_state: &GameState) {
        self.pathfinder().reset_edges(&game_state.world);
    }
}

impl<T> GameEventConsumer for PathfinderUpdater<T>
where
    T: TravelDuration + Sync + 'static,
{
    fn name(&self) -> &'static str {
        "pathfinder_service_event_consumer"
    }

    fn consume_game_event(&mut self, game_state: &GameState, event: &GameEvent) -> CaptureEvent {
        if let GameEvent::Init = event {
            self.reset_pathfinder(game_state);
        }
        CaptureEvent::No
    }

    fn consume_engine_event(&mut self, _: &GameState, _: Arc<Event>) -> CaptureEvent {
        CaptureEvent::No
    }
}
