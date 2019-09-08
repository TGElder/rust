use super::*;
use crate::visibility_computer::*;
use commons::grid::*;
use commons::v2;
use isometric::coords::*;

pub struct VisibilityHandler {
    command_tx: Sender<GameCommand>,
    visibility_computer: VisibilityComputer,
}

impl VisibilityHandler {
    pub fn new(command_tx: Sender<GameCommand>) -> VisibilityHandler {
        VisibilityHandler {
            command_tx,
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
        self.command_tx
            .send(GameCommand::RevealCells(CellSelection::Some(newly_visible)))
            .unwrap();
    }

    fn check_visited(&mut self, game_state: &GameState) {
        if let Some(WorldCoord { x, y, .. }) = game_state
            .avatar_state
            .compute_world_coord(&game_state.world, &game_state.game_micros)
        {
            let position = v2(x.round() as usize, y.round() as usize);
            if let Some(cell) = game_state.world.get_cell(&position) {
                if !cell.visited {
                    self.command_tx
                        .send(GameCommand::VisitCells(CellSelection::Some(vec![position])))
                        .unwrap();
                }
            }
        }
    }
}

impl GameEventConsumer for VisibilityHandler {
    fn consume_game_event(&mut self, game_state: &GameState, event: &GameEvent) -> CaptureEvent {
        if let GameEvent::CellsVisited(CellSelection::Some(cells)) = event {
            self.check_visibility(game_state, cells);
        }
        CaptureEvent::No
    }

    fn consume_engine_event(&mut self, game_state: &GameState, event: Arc<Event>) -> CaptureEvent {
        if let Event::Tick = *event {
            self.check_visited(game_state);
        }
        CaptureEvent::No
    }
}
