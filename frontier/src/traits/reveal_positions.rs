use std::collections::HashSet;

use commons::async_trait::async_trait;
use commons::{FutureExt, Grid, V2};

use crate::traits::{
    Micros, PathfinderWithPlannedRoads, PathfinderWithoutPlannedRoads, Redraw, SendGame, SendSim,
    SendVoyager, SendWorld, UpdatePathfinderPositions,
};
use crate::world::World;

#[async_trait]
pub trait RevealPositions {
    async fn reveal_positions(&self, positions: HashSet<V2<usize>>, revealed_by: &'static str);
}

#[async_trait]
impl<T> RevealPositions for T
where
    T: Micros
        + PathfinderWithPlannedRoads
        + PathfinderWithoutPlannedRoads
        + Redraw
        + SendGame
        + SendSim
        + SendVoyager
        + SendWorld,
{
    async fn reveal_positions(&self, positions: HashSet<V2<usize>>, revealed_by: &'static str) {
        let newly_visible = send_set_visible_get_newly_visible(self, positions).await;
        update_visible_land_positions(self, newly_visible.len()).await;
        voyage(self, newly_visible.clone(), revealed_by);
        update_sim(self, newly_visible.clone());
        join!(
            redraw(self, &newly_visible),
            self.update_pathfinder_positions(&newly_visible),
        );
    }
}

async fn send_set_visible_get_newly_visible<T>(
    x: &T,
    positions: HashSet<V2<usize>>,
) -> HashSet<V2<usize>>
where
    T: SendWorld,
{
    x.send_world(move |world| set_visible_get_newly_visible(world, positions))
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

async fn update_visible_land_positions<T>(x: &T, newly_visible_count: usize)
where
    T: SendGame,
{
    x.send_game(move |game| game.mut_state().visible_land_positions += newly_visible_count)
        .await
}

fn voyage<T>(x: &T, positions: HashSet<V2<usize>>, revealed_by: &'static str)
where
    T: SendVoyager,
{
    x.send_voyager_future_background(move |voyager| {
        voyager.voyage_to(positions, revealed_by).boxed()
    });
}

fn update_sim<T>(x: &T, positions: HashSet<V2<usize>>)
where
    T: SendSim,
{
    x.send_sim_background(move |sim| {
        sim.refresh_positions(positions);
        sim.update_homeland_population();
    });
}

async fn redraw<T>(x: &T, positions: &HashSet<V2<usize>>)
where
    T: Micros + Redraw,
{
    let micros = x.micros().await;
    for position in positions {
        x.redraw_tile_at(*position, micros);
    }
}
