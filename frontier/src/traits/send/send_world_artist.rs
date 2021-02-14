use crate::actors::WorldArtistActor;
use crate::traits::{Micros, SendSettlements, WithWorld};
use futures::future::BoxFuture;

use super::SendTerritory;

pub trait SendWorldArtist:
    Micros + SendSettlements + SendTerritory + WithWorld + Send + Sync
{
    fn send_world_artist_future_background<F, O>(&self, function: F)
    where
        O: Send + 'static,
        F: FnOnce(&mut WorldArtistActor<Self>) -> BoxFuture<O> + Send + 'static;
}
