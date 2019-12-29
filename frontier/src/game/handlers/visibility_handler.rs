use super::*;
use crate::visibility_computer::*;
use commons::grid::*;

pub struct VisibilityHandler {
    command_tx: Sender<GameCommand>,
    visibility_computer: VisibilityComputer,
    visited_matrix: Option<M<bool>>,
}

impl VisibilityHandler {
    pub fn new(command_tx: Sender<GameCommand>) -> VisibilityHandler {
        VisibilityHandler {
            command_tx,
            visibility_computer: VisibilityComputer::default(),
            visited_matrix: None,
        }
    }

    fn init(&mut self, game_state: &GameState) {
        let world = &game_state.world;
        let width = world.width();
        let height = world.height();
        self.visited_matrix = Some(M::from_element(width, height, false));
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
        self.command_tx
            .send(GameCommand::RevealCells(CellSelection::Some(newly_visible)))
            .unwrap();
    }
}

impl GameEventConsumer for VisibilityHandler {
    fn consume_game_event(&mut self, game_state: &GameState, event: &GameEvent) -> CaptureEvent {
        match event {
            GameEvent::Init => self.init(game_state),
            GameEvent::CellsVisited(CellSelection::Some(cells)) => {
                self.check_visibility(game_state, cells)
            }
            _ => (),
        }
        CaptureEvent::No
    }

    fn consume_engine_event(&mut self, _: &GameState, _: Arc<Event>) -> CaptureEvent {
        CaptureEvent::No
    }
}
