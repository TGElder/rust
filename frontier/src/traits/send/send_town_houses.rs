use crate::actors::TownHouses;
use crate::traits::{GetNationDescription, GetSettlement, SendParameters, SendWorld, Settlements};
use commons::async_trait::async_trait;
use commons::future::BoxFuture;

#[async_trait]
pub trait SendTownHouses:
    GetNationDescription + GetSettlement + SendWorld + SendParameters + Settlements + Send + Sync
{
    fn send_town_houses_future_background<F, O>(&self, function: F)
    where
        O: Send + 'static,
        F: FnOnce(&mut TownHouses<Self>) -> BoxFuture<O> + Send + 'static;
}
