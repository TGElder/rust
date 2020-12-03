use std::sync::Arc;

use commons::futures::FutureExt;
use isometric::Event;

use crate::game::{CaptureEvent, GameEvent, GameEventConsumer, GameState};
use crate::traits::SendTownHouses;

const NAME: &str = "town_artist_forwarder";

pub struct TownArtistForwarder<X> {
    pub x: X,
}

impl<X> GameEventConsumer for TownArtistForwarder<X>
where
    X: SendTownHouses,
{
    fn name(&self) -> &'static str {
        NAME
    }

    fn consume_game_event(&mut self, _: &GameState, event: &GameEvent) -> CaptureEvent {
        match event {
            GameEvent::SettlementUpdated(settlement) => {
                let settlement = settlement.clone();
                self.x
                    .send_town_houses_future_background(move |town_houses| {
                        town_houses.update_settlement(settlement).boxed()
                    })
            }
            _ => (),
        }
        CaptureEvent::No
    }

    fn consume_engine_event(&mut self, _: &GameState, _: Arc<Event>) -> CaptureEvent {
        CaptureEvent::No
    }
}
