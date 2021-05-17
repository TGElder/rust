use commons::async_trait::async_trait;
use commons::edge::Edge;
use std::collections::{HashMap, HashSet};
use std::time::Duration;

use commons::V2;

use crate::bridge::Bridges;
use crate::pathfinder::ClosestTargetResult;
use crate::traits::{
    PathfinderForPlayer, PathfinderForRoutes, RunInBackground, WithPathfinder, WithWorld,
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
pub trait UpdatePathfinderEdges {
    async fn update_pathfinder_edges<P, I>(&self, pathfinder: &P, edges: I)
    where
        P: WithPathfinder + Clone + Send + Sync + 'static,
        I: IntoIterator<Item = EdgeDuration> + Send + Sync + 'static;
}

#[async_trait]
impl<T> UpdatePathfinderEdges for T
where
    T: RunInBackground + Send + Sync,
{
    async fn update_pathfinder_edges<P, I>(&self, pathfinder: &P, edges: I)
    where
        P: WithPathfinder + Clone + Send + Sync + 'static,
        I: IntoIterator<Item = EdgeDuration> + Send + Sync + 'static,
    {
        let pathfinder = (*pathfinder).clone();
        let pathfinder_future = async move {
            pathfinder
                .mut_pathfinder(move |pathfinder| {
                    for EdgeDuration { from, to, duration } in edges {
                        match duration {
                            Some(duration) => pathfinder.set_edge_duration(&from, &to, &duration),
                            None => pathfinder.remove_edge(&from, &to),
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
    T: PathfinderForPlayer + PathfinderForRoutes + UpdatePathfinderPositions + Send + Sync,
{
    async fn update_positions_all_pathfinders<I>(&self, positions: I)
    where
        I: IntoIterator<Item = V2<usize>> + Clone + Send + Sync + 'static,
    {
        let player_pathfinder = self.player_pathfinder();
        let routes_pathfinder = self.routes_pathfinder();

        join!(
            self.update_pathfinder_positions(player_pathfinder, positions.clone()),
            self.update_pathfinder_positions(routes_pathfinder, positions),
        );
    }
}

#[async_trait]
pub trait UpdateEdgesAllPathfinders {
    async fn update_edges_all_pathfinders<I>(&self, edges: I)
    where
        I: IntoIterator<Item = EdgeDuration> + Clone + Send + Sync + 'static;
}

#[async_trait]
impl<T> UpdateEdgesAllPathfinders for T
where
    T: PathfinderForPlayer + PathfinderForRoutes + UpdatePathfinderEdges + Send + Sync,
{
    async fn update_edges_all_pathfinders<I>(&self, edges: I)
    where
        I: IntoIterator<Item = EdgeDuration> + Clone + Send + Sync + 'static,
    {
        let player_pathfinder = self.player_pathfinder();
        let routes_pathfinder = self.routes_pathfinder();

        join!(
            self.update_pathfinder_edges(player_pathfinder, edges.clone()),
            self.update_pathfinder_edges(routes_pathfinder, edges),
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
pub trait LoadTargets {
    async fn load_targets<'a, I>(&self, targets: I)
    where
        I: Iterator<Item = Target<'a>> + Send;
}

pub struct Target<'a> {
    pub name: &'a str,
    pub position: &'a V2<usize>,
    pub target: bool,
}

#[async_trait]
impl<T> LoadTargets for T
where
    T: WithPathfinder + Sync,
{
    async fn load_targets<'a, I>(&self, targets: I)
    where
        I: Iterator<Item = Target<'a>> + Send,
    {
        self.mut_pathfinder(|pathfinder| {
            for Target {
                name,
                position,
                target,
            } in targets
            {
                pathfinder.load_target(name, position, target);
            }
        })
        .await;
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

#[async_trait]
pub trait CostOfPath {
    async fn cost_of_path<D>(
        &self,
        travel_duration: &D,
        bridges: &Bridges,
        path: &[V2<usize>],
    ) -> Option<Duration>
    where
        D: TravelDuration;
}

#[async_trait]
impl<T> CostOfPath for T
where
    T: WithWorld + Sync,
{
    async fn cost_of_path<D>(
        &self,
        travel_duration: &D,
        bridges: &Bridges,
        path: &[V2<usize>],
    ) -> Option<Duration>
    where
        D: TravelDuration,
    {
        self.with_world(|world| {
            (0..path.len() - 1)
                .map(|i| {
                    travel_duration
                        .get_duration(world, &path[i], &path[i + 1])
                        .or_else(|| {
                            bridges
                                .get(&Edge::new(path[i], path[i + 1]))
                                .map(|bridge| bridge.duration)
                        })
                })
                .sum()
        })
        .await
    }
}
