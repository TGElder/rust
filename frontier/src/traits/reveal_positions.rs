use std::collections::HashSet;

use commons::async_trait::async_trait;
use commons::{FutureExt, Grid, V2};

use crate::traits::{
    Micros, PathfinderWithPlannedRoads, PathfinderWithoutPlannedRoads, Redraw, SendGame,
    SendPathfinder, SendSim, SendVoyager, SendWorld,
};
use crate::travel_duration::{EdgeDuration, TravelDuration};
use crate::world::World;

#[async_trait]
pub trait RevealPositions {
    async fn reveal_positions(&self, cells: HashSet<V2<usize>>, revealed_by: &'static str);
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
            update_pathfinders(self, &newly_visible),
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
    x.send_sim_background(move |sim| sim.reveal_positions(positions));
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

async fn update_pathfinders<T>(x: &T, positions: &HashSet<V2<usize>>)
where
    T: PathfinderWithPlannedRoads + PathfinderWithoutPlannedRoads + SendWorld,
{
    let pathfinder_with = x.pathfinder_with_planned_roads().clone();
    let pathfinder_without = x.pathfinder_without_planned_roads().clone();

    join!(
        update_pathfinder(x, pathfinder_with, positions.clone()),
        update_pathfinder(x, pathfinder_without, positions.clone()),
    );
}

async fn update_pathfinder<T, P>(tx: &T, pathfinder: P, positions: HashSet<V2<usize>>)
where
    T: SendWorld,
    P: SendPathfinder + Send,
{
    let travel_duration = pathfinder
        .send_pathfinder(|pathfinder| pathfinder.travel_duration().clone())
        .await;

    let durations: HashSet<EdgeDuration> = tx
        .send_world(move |world| {
            positions
                .iter()
                .flat_map(|position| travel_duration.get_durations_for_position(world, &position))
                .collect()
        })
        .await;

    pathfinder.send_pathfinder_background(move |pathfinder| {
        for EdgeDuration { from, to, duration } in durations {
            if let Some(duration) = duration {
                pathfinder.set_edge_duration(&from, &to, &duration)
            }
        }
    });
}
