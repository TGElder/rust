use std::collections::HashSet;

use crate::traits::send::SendWorldArtist;
use crate::traits::Micros;
use commons::async_trait::async_trait;
use commons::V2;
use futures::FutureExt;

#[async_trait]
pub trait DrawWorld {
    fn draw_world_at(&self, when: u128);
    async fn draw_world(&self);
    fn draw_world_tiles_at(&self, tiles: HashSet<V2<usize>>, when: u128);
    async fn draw_world_tiles(&self, tiles: HashSet<V2<usize>>);
}

#[async_trait]
impl<T> DrawWorld for T
where
    T: Micros + SendWorldArtist,
{
    fn draw_world_at(&self, when: u128) {
        self.send_world_artist_future_background(move |world_artist| {
            world_artist.redraw_all_at(when).boxed()
        });
    }

    async fn draw_world(&self) {
        let when = self.micros().await;
        self.draw_world_at(when);
    }

    fn draw_world_tiles_at(&self, tiles: HashSet<V2<usize>>, when: u128) {
        self.send_world_artist_future_background(move |world_artist| {
            world_artist.redraw_tiles_at(tiles, when).boxed()
        });
    }

    async fn draw_world_tiles(&self, tiles: HashSet<V2<usize>>) {
        let when = self.micros().await;
        self.draw_world_tiles_at(tiles, when);
    }
}
