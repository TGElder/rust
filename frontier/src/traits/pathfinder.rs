use commons::async_trait::async_trait;
use commons::log::info;
use std::collections::{HashMap, HashSet};
use std::time::Duration;

use commons::V2;

use crate::pathfinder::ClosestTargetResult;
use crate::traits::{
    PathfinderWithPlannedRoads, PathfinderWithoutPlannedRoads, RunInBackground, WithPathfinder,
    WithWorld,
};
use crate::travel_duration::{EdgeDuration, TravelDuration};

#[async_trait]
pub trait FindPath {
    async fn find_path(&self, from: &[V2<usize>], to: &[V2<usize>]) -> Option<Vec<V2<usize>>>;
}

#[async_trait]
impl<T> FindPath for T
where
    T: WithPathfinder + Sync,
{
    async fn find_path(&self, from: &[V2<usize>], to: &[V2<usize>]) -> Option<Vec<V2<usize>>> {
        self.with_pathfinder(|pathfinder| pathfinder.find_path(from, to))
            .await
    }
}

#[async_trait]
pub trait InBounds {
    async fn in_bounds(&self, position: &V2<usize>) -> bool;
}

#[async_trait]
impl<T> InBounds for T
where
    T: WithPathfinder + Sync,
{
    async fn in_bounds(&self, position: &V2<usize>) -> bool {
        self.with_pathfinder(|pathfinder| pathfinder.in_bounds(position))
            .await
    }
}

#[async_trait]
pub trait LowestDuration {
    async fn lowest_duration(&self, path: &[V2<usize>]) -> Option<Duration>;
}

#[async_trait]
impl<T> LowestDuration for T
where
    T: WithPathfinder + Sync,
{
    async fn lowest_duration(&self, path: &[V2<usize>]) -> Option<Duration> {
        self.with_pathfinder(|pathfinder| pathfinder.lowest_duration(path))
            .await
    }
}

#[async_trait]
pub trait PositionsWithin {
    async fn positions_within(
        &self,
        positions: &[V2<usize>],
        duration: &Duration,
    ) -> HashMap<V2<usize>, Duration>;
}

#[async_trait]
impl<T> PositionsWithin for T
where
    T: WithPathfinder + Sync,
{
    async fn positions_within(
        &self,
        positions: &[V2<usize>],
        duration: &Duration,
    ) -> HashMap<V2<usize>, Duration> {
        self.with_pathfinder(|pathfinder| pathfinder.positions_within(positions, duration))
            .await
    }
}

#[async_trait]
pub trait UpdatePathfinderPositions {
    async fn update_pathfinder_positions<P, I>(&self, pathfinder: &P, positions: I)
    where
        P: WithPathfinder + Clone + Send + Sync + 'static,
        I: IntoIterator<Item = V2<usize>> + Send + Sync + 'static;
}

#[async_trait]
impl<T> UpdatePathfinderPositions for T
where
    T: RunInBackground + WithWorld + Send + Sync,
{
    async fn update_pathfinder_positions<P, I>(&self, pathfinder: &P, positions: I)
    where
        P: WithPathfinder + Clone + Send + Sync + 'static,
        I: IntoIterator<Item = V2<usize>> + Send + Sync + 'static,
    {
        let travel_duration = pathfinder
            .with_pathfinder(|pathfinder| pathfinder.travel_duration().clone())
            .await;

        let durations: HashSet<EdgeDuration> = self
            .with_world(|world| {
                positions
                    .into_iter()
                    .flat_map(|position| {
                        travel_duration.get_durations_for_position(world, position)
                    })
                    .collect()
            })
            .await;

        let pathfinder = (*pathfinder).clone();
        let pathfinder_future = async move {
            pathfinder
                .mut_pathfinder(move |pathfinder| {
                    for EdgeDuration { from, to, duration } in durations {
                        if let Some(duration) = duration {
                            pathfinder.set_edge_duration(&from, &to, &duration)
                        }
                    }
                })
                .await;
        };
        self.run_in_background(pathfinder_future);
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
    T: WithPathfinder + Sync,
{
    async fn init_targets(&self, name: String) {
        self.mut_pathfinder(move |pathfinder| pathfinder.init_targets(name))
            .await
    }
}

#[async_trait]
pub trait LoadTarget {
    async fn load_target(&self, name: &str, position: &V2<usize>, target: bool);
}

#[async_trait]
impl<T> LoadTarget for T
where
    T: WithPathfinder + Sync,
{
    async fn load_target(&self, name: &str, position: &V2<usize>, target: bool) {
        self.mut_pathfinder(move |pathfinder| pathfinder.load_target(&name, position, target))
            .await
    }
}

#[async_trait]
pub trait ClosestTargets {
    async fn closest_targets(
        &self,
        positions: &[V2<usize>],
        targets: &str,
        n_closest: usize,
    ) -> Vec<ClosestTargetResult>;
}

#[async_trait]
impl<T> ClosestTargets for T
where
    T: WithPathfinder + Sync,
{
    async fn closest_targets(
        &self,
        positions: &[V2<usize>],
        targets: &str,
        n_closest: usize,
    ) -> Vec<ClosestTargetResult> {
        self.with_pathfinder(|pathfinder| pathfinder.closest_targets(positions, targets, n_closest))
            .await
    }
}
