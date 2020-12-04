use std::sync::Arc;

use commons::futures::FutureExt;
use isometric::Event;

use crate::game::{CaptureEvent, GameEvent, GameEventConsumer, GameState};
use crate::traits::{SendTownHouseArtist, SendTownLabelArtist};

const NAME: &str = "town_artist_forwarder";

pub struct TownArtistForwarder<X> {
    pub x: X,
}

impl<X> GameEventConsumer for TownArtistForwarder<X>
where
    X: SendTownHouseArtist + SendTownLabelArtist,
{
    fn name(&self) -> &'static str {
        NAME
    }

    fn consume_game_event(&mut self, _: &GameState, event: &GameEvent) -> CaptureEvent {
        if let GameEvent::SettlementUpdated(settlement) = event {
            let house_settlement = settlement.clone();
            self.x
                .send_town_house_artist_future_background(move |town_house_artist| {
                    town_house_artist
                        .update_settlement(house_settlement)
                        .boxed()
                });
            let label_settlement = settlement.clone();
            self.x
                .send_town_label_artist_future_background(move |town_label_artist| {
                    town_label_artist.update_label(label_settlement).boxed()
                });
        }
        CaptureEvent::No
    }

    fn consume_engine_event(&mut self, _: &GameState, _: Arc<Event>) -> CaptureEvent {
        CaptureEvent::No
    }
}
