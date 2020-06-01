use crate::game::*;
use commons::grid::Grid;
use commons::V2;
use isometric::Event;
use std::sync::Arc;

use crate::game_event_consumers::VisibilityHandlerMessage;
use std::sync::mpsc::Sender;

const HANDLE: &str = "visibility_from_towns";

pub struct VisibilityFromTowns {
    tx: Sender<VisibilityHandlerMessage>,
}

impl VisibilityFromTowns {
    pub fn new(tx: &Sender<VisibilityHandlerMessage>) -> VisibilityFromTowns {
        VisibilityFromTowns { tx: tx.clone() }
    }

    fn tick(&mut self, game_state: &GameState) {
        self.tx
            .send(VisibilityHandlerMessage {
                visited: town_visited_cells(game_state).collect(),
            })
            .unwrap();
    }
}

fn town_visited_cells<'a>(game_state: &'a GameState) -> impl Iterator<Item = V2<usize>> + 'a {
    let world = &game_state.world;
    game_state
        .settlements
        .keys()
        .flat_map(move |town| world.get_corners_in_bounds(town))
}

impl GameEventConsumer for VisibilityFromTowns {
    fn name(&self) -> &'static str {
        HANDLE
    }

    fn consume_game_event(&mut self, game_state: &GameState, event: &GameEvent) -> CaptureEvent {
        match event {
            GameEvent::Tick { .. } => self.tick(game_state),
            _ => (),
        }
        CaptureEvent::No
    }

    fn consume_engine_event(&mut self, _: &GameState, _: Arc<Event>) -> CaptureEvent {
        CaptureEvent::No
    }
}
