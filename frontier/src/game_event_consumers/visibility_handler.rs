use super::*;
use crate::visibility_computer::*;

const HANDLE: &str = "visibility_handler";

pub struct VisibilityHandler {
    game_tx: UpdateSender<Game>,
    visibility_computer: VisibilityComputer,
}

impl VisibilityHandler {
    pub fn new(game_tx: &UpdateSender<Game>) -> VisibilityHandler {
        VisibilityHandler {
            game_tx: game_tx.clone_with_handle(HANDLE),
            visibility_computer: VisibilityComputer::default(),
        }
    }

    fn check_visibility(&mut self, game_state: &GameState, cells: &[V2<usize>]) {
        let mut newly_visible = vec![];
        for cell in cells {
            newly_visible.append(
                &mut self
                    .visibility_computer
                    .get_newly_visible_from(&game_state.world, *cell),
            );
        }

        self.game_tx
            .update(move |game: &mut Game| game.reveal_cells(newly_visible));
    }
}

impl GameEventConsumer for VisibilityHandler {
    fn name(&self) -> &'static str {
        HANDLE
    }

    fn consume_game_event(&mut self, game_state: &GameState, event: &GameEvent) -> CaptureEvent {
        if let GameEvent::CellsVisited(CellSelection::Some(cells)) = event {
            self.check_visibility(game_state, cells)
        }
        CaptureEvent::No
    }

    fn consume_engine_event(&mut self, _: &GameState, _: Arc<Event>) -> CaptureEvent {
        CaptureEvent::No
    }

    fn shutdown(&mut self) {}

    fn is_shutdown(&self) -> bool {
        true
    }
}
