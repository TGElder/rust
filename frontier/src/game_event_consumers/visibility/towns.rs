use crate::game::*;
use crate::settlement::SettlementClass;
use commons::grid::Grid;
use commons::V2;
use isometric::Event;
use std::sync::Arc;

use crate::actors::VisibilityHandlerMessage;
use commons::async_channel::Sender;
use commons::futures::executor::block_on;

const HANDLE: &str = "visibility_from_towns";

pub struct VisibilityFromTowns {
    tx: Sender<VisibilityHandlerMessage>,
}

impl VisibilityFromTowns {
    pub fn new(tx: &Sender<VisibilityHandlerMessage>) -> VisibilityFromTowns {
        VisibilityFromTowns { tx: tx.clone() }
    }

    fn tick(&mut self, game_state: &GameState) {
        block_on(self.tx.send(VisibilityHandlerMessage {
            visited: town_visited_cells(game_state).collect(),
        }))
        .unwrap();
    }
}

fn town_visited_cells<'a>(game_state: &'a GameState) -> impl Iterator<Item = V2<usize>> + 'a {
    let world = &game_state.world;
    game_state
        .settlements
        .iter()
        .filter(|(_, settlement)| settlement.class == SettlementClass::Town)
        .flat_map(move |(position, _)| world.get_corners_in_bounds(position))
}

impl GameEventConsumer for VisibilityFromTowns {
    fn name(&self) -> &'static str {
        HANDLE
    }

    fn consume_game_event(&mut self, game_state: &GameState, event: &GameEvent) -> CaptureEvent {
        if let GameEvent::Tick { .. } = event {
            self.tick(game_state);
        }
        CaptureEvent::No
    }

    fn consume_engine_event(&mut self, _: &GameState, _: Arc<Event>) -> CaptureEvent {
        CaptureEvent::No
    }
}
