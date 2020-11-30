use futures::FutureExt;

use crate::traits::SendWorldArtist;

use super::*;

const NAME: &str = "world_artist_handler";

pub struct WorldArtistHandler<T> {
    x: T,
}

impl<T> WorldArtistHandler<T>
where
    T: SendWorldArtist,
{
    pub fn new(x: T) -> WorldArtistHandler<T> {
        WorldArtistHandler { x }
    }

    fn update_cells(&mut self, game_state: &GameState, cells: &[V2<usize>]) {
        for cell in cells {
            let cell = *cell;
            let when = game_state.game_micros;
            self.x.send_world_artist_future_background(move |artist| {
                artist.redraw_tile_at(cell, when).boxed()
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

impl<T> GameEventConsumer for WorldArtistHandler<T>
where
    T: SendWorldArtist,
{
    fn name(&self) -> &'static str {
        NAME
    }

    fn consume_game_event(&mut self, game_state: &GameState, event: &GameEvent) -> CaptureEvent {
        match event {
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
