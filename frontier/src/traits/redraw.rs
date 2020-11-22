use crate::traits::SendWorldArtist;
use commons::async_trait::async_trait;
use commons::future::FutureExt;
use commons::V2;

#[async_trait]
pub trait Redraw {
    fn redraw_all_at(&mut self, when: u128);
    fn redraw_tile_at(&mut self, tile: V2<usize>, when: u128);
}

#[async_trait]
impl<T> Redraw for T
where
    T: SendWorldArtist,
{
    fn redraw_all_at(&mut self, when: u128) {
        self.send_world_artist_future_background(move |world_artist| {
            world_artist.redraw_all_at(when).boxed()
        });
    }

    fn redraw_tile_at(&mut self, tile: V2<usize>, when: u128) {
        self.send_world_artist_future_background(move |world_artist| {
            world_artist.redraw_tile_at(tile, when).boxed()
        });
    }
}
