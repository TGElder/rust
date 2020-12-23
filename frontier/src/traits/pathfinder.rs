use commons::async_trait::async_trait;
use std::collections::{HashMap, HashSet};
use std::time::Duration;

use commons::V2;

use crate::traits::{
    PathfinderWithPlannedRoads, PathfinderWithoutPlannedRoads, SendPathfinder, SendWorld,
};
use crate::travel_duration::{EdgeDuration, TravelDuration};

#[async_trait]
pub trait PositionsWithin {
    async fn positions_within(
        &self,
        positions: Vec<V2<usize>>,
        duration: Duration,
    ) -> HashMap<V2<usize>, Duration>;
}

#[async_trait]
impl<T> PositionsWithin for T
where
    T: SendPathfinder + Sync,
{
    async fn positions_within(
        &self,
        positions: Vec<V2<usize>>,
        duration: Duration,
    ) -> HashMap<V2<usize>, Duration> {
        self.send_pathfinder(move |pathfinder| pathfinder.positions_within(&positions, &duration))
            .await
    }
}

#[async_trait]
pub trait UpdatePathfinderPositions {
    async fn update_pathfinder_positions<P>(&self, pathfinder: P, positions: Vec<V2<usize>>)
    where
        P: SendPathfinder + Send + Sync;
}

#[async_trait]
impl<T> UpdatePathfinderPositions for T
where
    T: SendWorld + Send + Sync,
{
    async fn update_pathfinder_positions<P>(&self, pathfinder: P, positions: Vec<V2<usize>>)
    where
        P: SendPathfinder + Send + Sync,
    {
        let travel_duration = pathfinder
            .send_pathfinder(|pathfinder| pathfinder.travel_duration().clone())
            .await;

        let durations: HashSet<EdgeDuration> = self
            .send_world(move |world| {
                positions
                    .iter()
                    .flat_map(|position| {
                        travel_duration.get_durations_for_position(world, &position)
                    })
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
}

#[async_trait]
pub trait UpdatePositionsAllPathfinders {
    async fn update_positions_all_pathfinders(&self, positions: Vec<V2<usize>>);
}

#[async_trait]
impl<T> UpdatePositionsAllPathfinders for T
where
    T: PathfinderWithPlannedRoads
        + PathfinderWithoutPlannedRoads
        + UpdatePathfinderPositions
        + Send
        + Sync,
{
    async fn update_positions_all_pathfinders(&self, positions: Vec<V2<usize>>) {
        let pathfinder_with = self.pathfinder_with_planned_roads().clone();
        let pathfinder_without = self.pathfinder_without_planned_roads().clone();

        join!(
            self.update_pathfinder_positions(pathfinder_with, positions.clone()),
            self.update_pathfinder_positions(pathfinder_without, positions),
        );
    }
}
