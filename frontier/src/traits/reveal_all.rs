use std::collections::HashSet;

use commons::async_trait::async_trait;
use commons::grid::Grid;
use commons::{v2, V2};

use crate::traits::{
    DrawWorld, Micros, RefreshPositions, SendWorld, UpdatePositionsAllPathfinders,
};

#[async_trait]
pub trait RevealAll {
    async fn reveal_all(&self);
}

#[async_trait]
impl<T> RevealAll for T
where
    T: DrawWorld
        + Micros
        + RefreshPositions
        + SendWorld
        + UpdatePositionsAllPathfinders
        + Send
        + Sync,
{
    async fn reveal_all(&self) {
        let (width, height) = reveal_all_get_dimensions(self).await;
        let positions = all_positions(width, height);
        self.refresh_positions(positions.clone()).await;
        join!(
            redraw_all(self),
            self.update_positions_all_pathfinders(positions)
        );
    }
}

async fn reveal_all_get_dimensions<T>(tx: &T) -> (usize, usize)
where
    T: SendWorld,
{
    tx.send_world(move |world| {
        world.reveal_all();
        (world.width(), world.height())
    })
    .await
}

fn all_positions(width: usize, height: usize) -> HashSet<V2<usize>> {
    (0..width)
        .flat_map(|x| (0..height).map(move |y| v2(x, y)))
        .collect()
}

async fn redraw_all<T>(tx: &T)
where
    T: DrawWorld + Micros,
{
    let micros = tx.micros().await;
    tx.draw_world(micros);
}
