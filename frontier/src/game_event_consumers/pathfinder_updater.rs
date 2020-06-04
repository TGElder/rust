use super::*;
use crate::pathfinder::*;
use crate::travel_duration::*;
use commons::Arm;
use std::sync::MutexGuard;

pub struct PathfinderUpdater<T>
where
    T: TravelDuration,
{
    pathfinder: Arm<Pathfinder<T>>,
}

impl<T> PathfinderUpdater<T>
where
    T: TravelDuration + Sync + 'static,
{
    pub fn new(pathfinder: Arm<Pathfinder<T>>) -> PathfinderUpdater<T> {
        PathfinderUpdater { pathfinder }
    }

    fn pathfinder(&mut self) -> MutexGuard<Pathfinder<T>> {
        self.pathfinder.lock().unwrap()
    }

    fn reset_pathfinder(&mut self, game_state: &GameState) {
        self.pathfinder().reset_edges(&game_state.world);
    }

    fn update_pathfinder_with_cells(&mut self, game_state: &GameState, cells: &[V2<usize>]) {
        for cell in cells {
            self.pathfinder().update_node(&game_state.world, cell);
        }
    }

    fn update_pathfinder_with_roads(&mut self, game_state: &GameState, result: &RoadBuilderResult) {
        result.update_pathfinder(&game_state.world, &mut self.pathfinder());
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
        match event {
            GameEvent::Init => self.reset_pathfinder(game_state),
            GameEvent::CellsRevealed { selection, .. } => {
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
}
