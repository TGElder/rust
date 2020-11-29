use std::collections::HashSet;

use commons::async_trait::async_trait;
use commons::{FutureExt, Grid, V2};

use crate::traits::{
    Micros, PathfinderWithPlannedRoads, PathfinderWithoutPlannedRoads, SendPathfinder, SendVoyager,
    SendWorld, SendWorldArtist,
};
use crate::travel_duration::{EdgeDuration, TravelDuration};
use crate::world::World;

#[async_trait]
pub trait RevealCells {
    async fn reveal_cells(&mut self, cells: Vec<V2<usize>>, revealed_by: &'static str);
}

#[async_trait]
impl<T> RevealCells for T
where
    T: Micros
        + SendVoyager
        + SendWorld
        + SendWorldArtist
        + PathfinderWithPlannedRoads
        + PathfinderWithoutPlannedRoads,
{
    async fn reveal_cells(&mut self, cells: Vec<V2<usize>>, revealed_by: &'static str) {
        let newly_visible = self
            .send_world(move |world| set_visible_get_newly_visible(world, cells))
            .await;
        let micros = self.micros().await;
        for cell in newly_visible.clone() {
            self.send_world_artist_future_background(move |artist| {
                artist.redraw_tile_at(cell, micros).boxed()
            });
        }

        let voyager_positions = newly_visible.clone();
        self.send_voyager_future_background(move |voyager| {
            voyager.voyage_to(voyager_positions, revealed_by).boxed()
        });

        let pathfinder_with = self.pathfinder_with_planned_roads().clone();
        let pathfinder_without = self.pathfinder_without_planned_roads().clone();

        join!(
            update_pathfinder_durations(self, pathfinder_with, newly_visible.clone()),
            update_pathfinder_durations(self, pathfinder_without, newly_visible),
        );
    }
}

fn set_visible_get_newly_visible(world: &mut World, cells: Vec<V2<usize>>) -> Vec<V2<usize>> {
    let mut out = vec![];
    for position in cells {
        if let Some(world_cell) = world.mut_cell(&position) {
            if !world_cell.visible {
                world_cell.visible = true;
                out.push(position);
            }
        }
    }
    out
}

async fn update_pathfinder_durations<T, P>(tx: &T, pathfinder: P, positions: Vec<V2<usize>>)
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
