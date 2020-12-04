use crate::actors::TownHouseArtist;
use crate::traits::{GetNationDescription, GetSettlement, SendWorld, Settlements};
use commons::async_trait::async_trait;
use commons::future::BoxFuture;

#[async_trait]
pub trait SendTownHouseArtist:
    GetNationDescription + GetSettlement + SendWorld + Settlements + Send + Sync
{
    fn send_town_house_artist_future_background<F, O>(&self, function: F)
    where
        O: Send + 'static,
        F: FnOnce(&mut TownHouseArtist<Self>) -> BoxFuture<O> + Send + 'static;
}
