use std::sync::Arc;

use isometric::Event;

use crate::game::{CaptureEvent, GameEvent, GameEventConsumer, GameState};
use crate::traits::DrawTown;

const NAME: &str = "town_artist_forwarder";

pub struct TownArtistForwarder<X> {
    pub x: X,
}

impl<X> GameEventConsumer for TownArtistForwarder<X>
where
    X: DrawTown + Send,
{
    fn name(&self) -> &'static str {
        NAME
    }

    fn consume_game_event(&mut self, _: &GameState, event: &GameEvent) -> CaptureEvent {
        if let GameEvent::SettlementUpdated(settlement) = event {
            self.x.draw_town(settlement.clone());
        }
        CaptureEvent::No
    }

    fn consume_engine_event(&mut self, _: &GameState, _: Arc<Event>) -> CaptureEvent {
        CaptureEvent::No
    }
}
