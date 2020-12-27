use std::collections::HashSet;

use commons::async_trait::async_trait;
use commons::grid::Grid;
use commons::V2;
use futures::FutureExt;

use crate::traits::{
    DrawWorld, Micros, SendGame, SendVoyager, SendWorld, UpdatePositionsAllPathfinders,
};
use crate::world::World;

#[async_trait]
pub trait RevealPositions {
    async fn reveal_positions(&self, positions: HashSet<V2<usize>>, revealed_by: &'static str);
}

#[async_trait]
impl<T> RevealPositions for T
where
    T: DrawWorld + Micros + SendGame + SendVoyager + SendWorld + UpdatePositionsAllPathfinders,
{
    async fn reveal_positions(&self, positions: HashSet<V2<usize>>, revealed_by: &'static str) {
        let newly_visible = send_set_visible_get_newly_visible(self, positions).await;
        update_visible_land_positions(self, newly_visible.len()).await;
        voyage(self, newly_visible.clone(), revealed_by);
        join!(
            redraw(self, &newly_visible),
            self.update_positions_all_pathfinders(newly_visible.clone()),
        );
    }
}

async fn send_set_visible_get_newly_visible<T>(
    tx: &T,
    positions: HashSet<V2<usize>>,
) -> HashSet<V2<usize>>
where
    T: SendWorld,
{
    tx.send_world(move |world| set_visible_get_newly_visible(world, positions))
        .await
}

fn set_visible_get_newly_visible(
    world: &mut World,
    positions: HashSet<V2<usize>>,
) -> HashSet<V2<usize>> {
    let mut out = hashset! {};
    for position in positions {
        if let Some(world_cell) = world.mut_cell(&position) {
            if !world_cell.visible {
                world_cell.visible = true;
                out.insert(position);
            }
        }
    }
    out
}

async fn update_visible_land_positions<T>(tx: &T, newly_visible_count: usize)
where
    T: SendGame,
{
    tx.send_game(move |game| game.mut_state().visible_land_positions += newly_visible_count)
        .await
}

fn voyage<T>(tx: &T, positions: HashSet<V2<usize>>, revealed_by: &'static str)
where
    T: SendVoyager,
{
    tx.send_voyager_future_background(move |voyager| {
        voyager.voyage_to(positions, revealed_by).boxed()
    });
}

async fn redraw<T>(tx: &T, positions: &HashSet<V2<usize>>)
where
    T: DrawWorld + Micros,
{
    let micros = tx.micros().await;
    for position in positions {
        tx.draw_world_tile(*position, micros);
    }
}
