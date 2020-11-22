use crate::actors::WorldArtistActor;
use commons::async_trait::async_trait;
use commons::future::BoxFuture;

#[async_trait]
pub trait WithWorldArtist {
    async fn send_world_artist_future<F, O>(&mut self, function: F) -> O
    where
        O: Send + 'static,
        F: FnOnce(&mut WorldArtistActor) -> BoxFuture<O> + Send + 'static;

    fn send_world_artist_future_background<F, O>(&mut self, function: F)
    where
        O: Send + 'static,
        F: FnOnce(&mut WorldArtistActor) -> BoxFuture<O> + Send + 'static;
}
