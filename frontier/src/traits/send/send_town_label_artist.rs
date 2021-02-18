use crate::actors::TownLabelArtist;
use crate::traits::{
    GetNationDescription, GetSettlement, SendEngineCommands, Settlements, WithWorld,
};
use commons::async_trait::async_trait;
use futures::future::BoxFuture;

#[async_trait]
pub trait SendTownLabelArtist:
    GetNationDescription + GetSettlement + SendEngineCommands + Settlements + WithWorld + Send + Sync
{
    fn send_town_label_artist_future_background<F, O>(&self, function: F)
    where
        O: Send + 'static,
        F: FnOnce(&mut TownLabelArtist<Self>) -> BoxFuture<O> + Send + 'static;
}
