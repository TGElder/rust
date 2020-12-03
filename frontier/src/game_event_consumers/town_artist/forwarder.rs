use std::sync::Arc;

use isometric::Event;

use crate::game::{CaptureEvent, GameEvent, GameEventConsumer, GameState};

const NAME: &str = "town_artist_forwarder";

pub struct TownArtistForwarder{

}

impl GameEventConsumer for TownArtistForwarder {
    fn name(&self) -> &'static str {
        NAME
    }

    fn consume_game_event(&mut self, game_state: &GameState, event: &GameEvent) -> CaptureEvent {
        
        CaptureEvent::No
    }

    fn consume_engine_event(&mut self, game_state: &GameState, event: Arc<Event>) -> CaptureEvent {
        
        CaptureEvent::No
    }
}