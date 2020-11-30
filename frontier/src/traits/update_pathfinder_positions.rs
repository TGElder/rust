use std::collections::HashSet;

use commons::async_trait::async_trait;
use commons::V2;

use crate::traits::{
    PathfinderWithPlannedRoads, PathfinderWithoutPlannedRoads, SendPathfinder, SendWorld,
};
use crate::travel_duration::{EdgeDuration, TravelDuration};

#[async_trait]
pub trait UpdatePathfinderPositions {
    async fn update_pathfinder_positions(&self, positions: &HashSet<V2<usize>>);
}

#[async_trait]
impl<T> UpdatePathfinderPositions for T
where
    T: PathfinderWithPlannedRoads + PathfinderWithoutPlannedRoads + SendWorld + Sync,
{
    async fn update_pathfinder_positions(&self, positions: &HashSet<V2<usize>>) {
        let pathfinder_with = self.pathfinder_with_planned_roads().clone();
        let pathfinder_without = self.pathfinder_without_planned_roads().clone();

        join!(
            update_pathfinder_positions(self, pathfinder_with, positions.clone()),
            update_pathfinder_positions(self, pathfinder_without, positions.clone()),
        );
    }
}

async fn update_pathfinder_positions<T, P>(tx: &T, pathfinder: P, positions: HashSet<V2<usize>>)
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
