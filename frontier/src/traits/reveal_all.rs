use std::collections::HashSet;

use commons::async_trait::async_trait;
use commons::{v2, Grid, V2};

use crate::traits::{
    Micros, PathfinderWithPlannedRoads, PathfinderWithoutPlannedRoads, Redraw, SendGame, SendSim,
    SendWorld, UpdatePathfinderPositions,
};

#[async_trait]
pub trait RevealAll {
    async fn reveal_all(&self);
}

#[async_trait]
impl<T> RevealAll for T
where
    T: Micros
        + PathfinderWithPlannedRoads
        + PathfinderWithoutPlannedRoads
        + Redraw
        + SendGame
        + SendSim
        + SendWorld
        + Sync,
{
    async fn reveal_all(&self) {
        let (width, height) = reveal_all_get_dimensions(self).await;
        set_visible_land_positions(self, width * height).await;
        let positions = all_positions(width, height);
        update_sim(self, positions.clone());
        join!(
            redraw_all(self),
            self.update_pathfinder_positions(&positions)
        );
    }
}

async fn reveal_all_get_dimensions<T>(x: &T) -> (usize, usize)
where
    T: SendWorld,
{
    x.send_world(move |world| {
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

async fn set_visible_land_positions<T>(x: &T, visible_positions: usize)
where
    T: SendGame,
{
    x.send_game(move |game| game.mut_state().visible_land_positions = visible_positions)
        .await
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

async fn redraw_all<T>(x: &T)
where
    T: Micros + Redraw,
{
    let micros = x.micros().await;
    x.redraw_all_at(micros);
}