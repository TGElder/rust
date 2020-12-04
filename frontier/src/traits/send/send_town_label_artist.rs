use crate::actors::TownLabelArtist;
use crate::traits::{GetNationDescription, GetSettlement, SendWorld, Settlements};
use commons::async_trait::async_trait;
use commons::future::BoxFuture;

#[async_trait]
pub trait SendTownLabelArtist:
    GetNationDescription + GetSettlement + SendWorld + Settlements + Send + Sync
{
    fn send_town_label_artist_future_background<F, O>(&self, function: F)
    where
        O: Send + 'static,
        F: FnOnce(&mut TownLabelArtist<Self>) -> BoxFuture<O> + Send + 'static;
}
