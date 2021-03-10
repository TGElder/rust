use std::collections::HashSet;

use commons::async_trait::async_trait;
use commons::grid::Grid;
use commons::log::debug;
use commons::V2;
use futures::FutureExt;

use crate::traits::{
    DrawWorld, Micros, RefreshPositionsBackground, SendVoyager, UpdatePositionsAllPathfinders,
    WithWorld,
};

#[async_trait]
pub trait RevealPositions {
    async fn reveal_positions<'a>(
        &'a self,
        positions: &'a HashSet<V2<usize>>,
        revealed_by: &'static str,
    );
}

#[async_trait]
impl<T> RevealPositions for T
where
    T: DrawWorld
        + Micros
        + RefreshPositionsBackground
        + SendVoyager
        + UpdatePositionsAllPathfinders
        + WithWorld,
{
    async fn reveal_positions<'a>(
        &'a self,
        positions: &'a HashSet<V2<usize>>,
        revealed_by: &'static str,
    ) {
        if positions.is_empty() {
            return;
        }

        debug!("Revealing from {} positions", positions.len());

        let newly_visible = get_newly_visible(self, &positions).await;
        if newly_visible.is_empty() {
            return;
        }
        set_visible(self, &newly_visible).await;

        join!(
            redraw(self, &newly_visible),
            self.update_positions_all_pathfinders(newly_visible.clone()),
        );

        voyage(self, newly_visible.clone(), revealed_by);

        self.refresh_positions_background(newly_visible.clone());
    }
}

async fn get_newly_visible<T>(cx: &T, visible: &HashSet<V2<usize>>) -> HashSet<V2<usize>>
where
    T: WithWorld,
{
    let mut out = hashset! {};
    cx.with_world(|world| {
        for position in visible {
            if let Some(world_cell) = world.get_cell(&position) {
                if !world_cell.visible {
                    out.insert(*position);
                }
            }
        }
    })
    .await;
    out
}

async fn set_visible<T>(cx: &T, visible: &HashSet<V2<usize>>)
where
    T: WithWorld,
{
    cx.mut_world(|world| {
        for position in visible {
            if let Some(world_cell) = world.mut_cell(&position) {
                if !world_cell.visible {
                    world_cell.visible = true;
                }
            }
        }
    })
    .await;
}

fn voyage<T>(cx: &T, positions: HashSet<V2<usize>>, revealed_by: &'static str)
where
    T: SendVoyager,
{
    cx.send_voyager_future_background(move |voyager| {
        voyager.voyage_to(positions, revealed_by).boxed()
    });
}

async fn redraw<T>(cx: &T, positions: &HashSet<V2<usize>>)
where
    T: DrawWorld + Micros,
{
    let micros = cx.micros().await;
    for position in positions {
        cx.draw_world_tile(*position, micros);
    }
}
