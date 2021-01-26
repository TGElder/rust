use commons::async_trait::async_trait;
use std::collections::{HashMap, HashSet};
use std::time::Duration;

use commons::V2;

use crate::pathfinder::ClosestTargetResult;
use crate::traits::{
    PathfinderWithPlannedRoads, PathfinderWithoutPlannedRoads, SendPathfinder, SendWorld,
};
use crate::travel_duration::{EdgeDuration, TravelDuration};

#[async_trait]
pub trait FindPath {
    async fn find_path(&self, from: Vec<V2<usize>>, to: Vec<V2<usize>>) -> Option<Vec<V2<usize>>>;
}

#[async_trait]
impl<T> FindPath for T
where
    T: SendPathfinder + Sync,
{
    async fn find_path(&self, from: Vec<V2<usize>>, to: Vec<V2<usize>>) -> Option<Vec<V2<usize>>> {
        self.send_pathfinder(move |pathfinder| pathfinder.find_path(&from, &to))
            .await
    }
}

#[async_trait]
pub trait InBounds {
    async fn in_bounds(&self, position: V2<usize>) -> bool;
}

#[async_trait]
impl<T> InBounds for T
where
    T: SendPathfinder + Sync,
{
    async fn in_bounds(&self, position: V2<usize>) -> bool {
        self.send_pathfinder(move |pathfinder| pathfinder.in_bounds(&position))
            .await
    }
}

#[async_trait]
pub trait LowestDuration {
    async fn lowest_duration(&self, path: Vec<V2<usize>>) -> Option<Duration>;
}

#[async_trait]
impl<T> LowestDuration for T
where
    T: SendPathfinder + Sync,
{
    async fn lowest_duration(&self, path: Vec<V2<usize>>) -> Option<Duration> {
        self.send_pathfinder(move |pathfinder| pathfinder.lowest_duration(&path))
            .await
    }
}

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
    async fn update_pathfinder_positions<P, I>(&self, pathfinder: &P, positions: I)
    where
        P: SendPathfinder + Send + Sync,
        I: IntoIterator<Item = V2<usize>> + Send + Sync + 'static;
}

#[async_trait]
impl<T> UpdatePathfinderPositions for T
where
    T: SendWorld + Send + Sync,
{
    async fn update_pathfinder_positions<P, I>(&self, pathfinder: &P, positions: I)
    where
        P: SendPathfinder + Send + Sync,
        I: IntoIterator<Item = V2<usize>> + Send + Sync + 'static,
    {
        let travel_duration = pathfinder
            .send_pathfinder(|pathfinder| pathfinder.travel_duration().clone())
            .await;

        let durations: HashSet<EdgeDuration> = self
            .send_world(move |world| {
                positions
                    .into_iter()
                    .flat_map(|position| {
                        travel_duration.get_durations_for_position(world, position)
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
    async fn update_positions_all_pathfinders<I>(&self, positions: I)
    where
        I: IntoIterator<Item = V2<usize>> + Clone + Send + Sync + 'static;
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
    async fn update_positions_all_pathfinders<I>(&self, positions: I)
    where
        I: IntoIterator<Item = V2<usize>> + Clone + Send + Sync + 'static,
    {
        let pathfinder_with = self.pathfinder_with_planned_roads();
        let pathfinder_without = self.pathfinder_without_planned_roads();

        join!(
            self.update_pathfinder_positions(pathfinder_with, positions.clone()),
            self.update_pathfinder_positions(pathfinder_without, positions),
        );
    }
}

#[async_trait]
pub trait InitTargets {
    async fn init_targets(&self, name: String);
}

#[async_trait]
impl<T> InitTargets for T
where
    T: SendPathfinder + Sync,
{
    async fn init_targets(&self, name: String) {
        self.send_pathfinder(move |pathfinder| pathfinder.init_targets(name))
            .await
    }
}

#[async_trait]
pub trait LoadTarget {
    async fn load_target(&self, name: String, position: V2<usize>, target: bool);
}

#[async_trait]
impl<T> LoadTarget for T
where
    T: SendPathfinder + Sync,
{
    async fn load_target(&self, name: String, position: V2<usize>, target: bool) {
        self.send_pathfinder(move |pathfinder| pathfinder.load_target(&name, &position, target))
            .await
    }
}

#[async_trait]
pub trait ClosestTargets {
    async fn closest_targets(
        &self,
        positions: Vec<V2<usize>>,
        targets: String,
        n_closest: usize,
    ) -> Vec<ClosestTargetResult>;
}

#[async_trait]
impl<T> ClosestTargets for T
where
    T: SendPathfinder + Sync,
{
    async fn closest_targets(
        &self,
        positions: Vec<V2<usize>>,
        targets: String,
        n_closest: usize,
    ) -> Vec<ClosestTargetResult> {
        self.send_pathfinder(move |pathfinder| {
            pathfinder.closest_targets(&positions, &targets, n_closest)
        })
        .await
    }
}
