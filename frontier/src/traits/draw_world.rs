use std::collections::HashSet;

use crate::traits::send::SendWorldArtist;
use commons::async_trait::async_trait;
use commons::V2;
use futures::FutureExt;

#[async_trait]
pub trait DrawWorld {
    fn draw_world(&self, when: u128);
    fn draw_world_tiles(&self, tiles: HashSet<V2<usize>>, when: u128);
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

    fn draw_world_tiles(&self, tiles: HashSet<V2<usize>>, when: u128) {
        self.send_world_artist_future_background(move |world_artist| {
            world_artist.redraw_tiles_at(tiles, when).boxed()
        });
    }
}
