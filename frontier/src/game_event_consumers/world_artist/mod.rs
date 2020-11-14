use crate::actors::WorldArtistActor;
use futures::FutureExt;

use super::*;

const HANDLE: &str = "world_artist_handler";

pub struct WorldArtistHandler {
    actor_tx: FnSender<WorldArtistActor>,
    thread_pool: ThreadPool,
}

impl WorldArtistHandler {
    pub fn new(
        actor_tx: &FnSender<WorldArtistActor>,
        thread_pool: ThreadPool,
    ) -> WorldArtistHandler {
        WorldArtistHandler {
            actor_tx: actor_tx.clone_with_name(HANDLE),
            thread_pool,
        }
    }

    fn draw_all(&mut self) {
        let actor_tx = self.actor_tx.clone();
        self.thread_pool
            .spawn_ok(actor_tx.send_future(|artist| artist.redraw_all().boxed()))
    }

    fn update_cells(&mut self, game_state: &GameState, cells: &[V2<usize>]) {
        for cell in cells {
            let actor_tx = self.actor_tx.clone();
            let cell = *cell;
            let when = game_state.game_micros;
            self.thread_pool.spawn_ok(
                actor_tx.send_future(move |artist| artist.redraw_tile(cell, when).boxed()),
            );
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
                    CellSelection::All => self.draw_all(),
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
