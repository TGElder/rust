use crate::actors::{Redraw, RedrawType};
use commons::async_channel::Sender;

use super::*;

const HANDLE: &str = "world_artist_handler";

pub struct WorldArtistHandler {
    actor_tx: Sender<Redraw>,
    thread_pool: ThreadPool,
}

impl WorldArtistHandler {
    pub fn new(actor_tx: &Sender<Redraw>, thread_pool: ThreadPool) -> WorldArtistHandler {
        WorldArtistHandler {
            actor_tx: actor_tx.clone(),
            thread_pool,
        }
    }

    fn draw_all(&mut self, game_state: &GameState) {
        let actor_tx = self.actor_tx.clone();
        let when = game_state.game_micros;
        self.thread_pool.spawn_ok(async move {
            actor_tx
                .send(Redraw {
                    redraw_type: RedrawType::All,
                    when,
                })
                .await
                .unwrap()
        })
    }

    fn update_cells(&mut self, game_state: &GameState, cells: &[V2<usize>]) {
        for cell in cells {
            let actor_tx = self.actor_tx.clone();
            let cell = *cell;
            let when = game_state.game_micros;
            self.thread_pool.spawn_ok(async move {
                actor_tx
                    .send(Redraw {
                        redraw_type: RedrawType::Tile(cell),
                        when,
                    })
                    .await
                    .unwrap()
            });
        }
    }

    fn draw_territory(&mut self, game_state: &GameState, changes: &[TerritoryChange]) {
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
