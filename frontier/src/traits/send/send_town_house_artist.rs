use crate::actors::TownHouseArtist;
use crate::traits::{GetNationDescription, GetSettlement, Settlements, WithWorld};
use commons::async_trait::async_trait;
use futures::future::BoxFuture;

#[async_trait]
pub trait SendTownHouseArtist:
    GetNationDescription + GetSettlement + Settlements + WithWorld + Send + Sync
{
    fn send_town_house_artist_future_background<F, O>(&self, function: F)
    where
        O: Send + 'static,
        F: FnOnce(&mut TownHouseArtist<Self>) -> BoxFuture<O> + Send + 'static;
}
