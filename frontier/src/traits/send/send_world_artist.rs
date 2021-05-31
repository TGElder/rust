use crate::actors::WorldArtistActor;
use crate::traits::has::HasParameters;
use crate::traits::GetNationDescription;
use crate::traits::WithControllers;
use crate::traits::{Micros, SendEngineCommands, WithResources, WithSettlements, WithWorld};
use futures::future::BoxFuture;

pub trait SendWorldArtist:
    GetNationDescription
    + HasParameters
    + Micros
    + SendEngineCommands
    + WithControllers
    + WithResources
    + WithSettlements
    + WithWorld
    + Send
    + Sync
{
    fn send_world_artist_future_background<F, O>(&self, function: F)
    where
        O: Send + 'static,
        F: FnOnce(&mut WorldArtistActor<Self>) -> BoxFuture<O> + Send + 'static;
}
