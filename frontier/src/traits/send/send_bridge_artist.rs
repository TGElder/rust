use crate::actors::BridgeArtistActor;
use crate::traits::{SendEngineCommands, WithBridges, WithWorld};
use commons::async_trait::async_trait;
use futures::future::BoxFuture;

#[async_trait]
pub trait SendBridgeArtistActor:
    SendEngineCommands + WithBridges + WithWorld + Send + Sync
{
    fn send_bridge_artist_future_background<F, O>(&self, function: F)
    where
        O: Send + 'static,
        F: FnOnce(&mut BridgeArtistActor<Self>) -> BoxFuture<O> + Send + 'static;
}
