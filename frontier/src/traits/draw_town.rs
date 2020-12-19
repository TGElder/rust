use futures::FutureExt;

use crate::settlement::Settlement;
use crate::traits::{SendTownHouseArtist, SendTownLabelArtist};

pub trait DrawTown {
    fn draw_town(&self, town: Settlement);
}

impl<T> DrawTown for T
where
    T: SendTownHouseArtist + SendTownLabelArtist,
{
    fn draw_town(&self, town: Settlement) {
        let house_town = town.clone();
        self.send_town_house_artist_future_background(move |town_house_artist| {
            town_house_artist.update_settlement(house_town).boxed()
        });
        let label_town = town;
        self.send_town_label_artist_future_background(move |town_label_artist| {
            town_label_artist.update_label(label_town).boxed()
        });
    }
}
