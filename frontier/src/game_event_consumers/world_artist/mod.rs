use crate::actors::{Redraw, RedrawType};
use commons::async_channel::Sender as AsyncSender;
use commons::futures::executor::block_on;

use super::*;

const HANDLE: &str = "world_artist_handler";

pub struct WorldArtistHandler {
    actor_tx: AsyncSender<Redraw>,
    territory_layer: bool,
}

impl WorldArtistHandler {
    pub fn new(actor_tx: &AsyncSender<Redraw>) -> WorldArtistHandler {
        WorldArtistHandler {
            actor_tx: actor_tx.clone(),
            territory_layer: true,
        }
    }

    fn draw_all(&mut self, game_state: &GameState) {
        block_on(self.actor_tx.send(Redraw {
            redraw_type: RedrawType::All,
            when: game_state.game_micros,
        }))
        .unwrap();
    }

    fn update_cells(&mut self, game_state: &GameState, cells: &[V2<usize>]) {
        for cell in cells {
            block_on(self.actor_tx.send(Redraw {
                redraw_type: RedrawType::Tile(*cell),
                when: game_state.game_micros,
            }))
            .unwrap();
        }
    }

    fn draw_territory(&mut self, game_state: &GameState, changes: &[TerritoryChange]) {
        if !self.territory_layer {
            return;
        }
        let affected: Vec<V2<usize>> = changes
            .iter()
            .flat_map(|change| game_state.world.expand_position(&change.position))
            .collect();
        self.update_cells(game_state, &affected);
    }
}

impl GameEventConsumer for WorldArtistHandler {
    fn name(&self) -> &'static str {
        HANDLE
    }

    fn consume_game_event(&mut self, game_state: &GameState, event: &GameEvent) -> CaptureEvent {
        match event {
            GameEvent::CellsRevealed { selection, .. } => {
                match selection {
                    CellSelection::All => self.draw_all(game_state),
                    CellSelection::Some(cells) => self.update_cells(game_state, &cells),
                };
            }
            GameEvent::RoadsUpdated(result) => self.update_cells(game_state, result.path()),
            GameEvent::TerritoryChanged(changes) => self.draw_territory(game_state, changes),
            GameEvent::ObjectUpdated(position) => self.update_cells(game_state, &[*position]),
            _ => (),
        }
        CaptureEvent::No
    }

    fn consume_engine_event(&mut self, _: &GameState, _: Arc<Event>) -> CaptureEvent {
        CaptureEvent::No
    }
}
