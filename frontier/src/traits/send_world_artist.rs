use crate::actors::WorldArtistActor;
use commons::future::BoxFuture;

pub trait SendWorldArtist {
    fn send_world_artist_future_background<F, O>(&self, function: F)
    where
        O: Send + 'static,
        F: FnOnce(&mut WorldArtistActor) -> BoxFuture<O> + Send + 'static;
}
