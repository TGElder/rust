use crate::traits::send::SendWorldArtist;
use commons::async_trait::async_trait;
use commons::future::FutureExt;
use commons::V2;

#[async_trait]
pub trait DrawWorld {
    fn draw_world(&self, when: u128);
    fn draw_world_tile(&self, tile: V2<usize>, when: u128);
}

#[async_trait]
impl<T> DrawWorld for T
where
    T: SendWorldArtist,
{
    fn draw_world(&self, when: u128) {
        self.send_world_artist_future_background(move |world_artist| {
            world_artist.redraw_all_at(when).boxed()
        });
    }

    fn draw_world_tile(&self, tile: V2<usize>, when: u128) {
        self.send_world_artist_future_background(move |world_artist| {
            world_artist.redraw_tile_at(tile, when).boxed()
        });
    }
}
