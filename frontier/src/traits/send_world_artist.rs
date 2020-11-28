use crate::actors::WorldArtistActor;
use crate::traits::Micros;
use crate::traits::SendGame;
use commons::future::BoxFuture;

pub trait SendWorldArtist: Micros + SendGame + Send + Sync {
    fn send_world_artist_future_background<F, O>(&self, function: F)
    where
        O: Send + 'static,
        F: FnOnce(&mut WorldArtistActor<Self>) -> BoxFuture<O> + Send + 'static;
}
