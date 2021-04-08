use crate::actors::WorldArtistActor;
use crate::traits::has::HasParameters;
use crate::traits::{
    Micros, SendEngineCommands, WithResources, WithSettlements, WithTerritory, WithWorld,
};
use futures::future::BoxFuture;

pub trait SendWorldArtist:
    HasParameters
    + Micros
    + SendEngineCommands
    + WithResources
    + WithSettlements
    + WithTerritory
    + WithWorld
    + Send
    + Sync
{
    fn send_world_artist_future_background<F, O>(&self, function: F)
    where
        O: Send + 'static,
        F: FnOnce(&mut WorldArtistActor<Self>) -> BoxFuture<O> + Send + 'static;
}
