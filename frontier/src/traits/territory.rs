use std::collections::{HashMap, HashSet};
use std::time::Duration;

use commons::async_trait::async_trait;
use commons::{get_corners, Grid, V2};

use crate::traits::{
    DrawWorld, Micros, PathfinderWithoutPlannedRoads, PositionsWithin, SendParameters,
    SendTerritory, SendWorld,
};

#[async_trait]
pub trait AddController {
    async fn add_controller(&self, controller: &V2<usize>);
}

#[async_trait]
impl<T> AddController for T
where
    T: SendTerritory + Sync,
{
    async fn add_controller(&self, controller: &V2<usize>) {
        let controller = *controller;
        self.send_territory(move |territory| territory.add_controller(controller))
            .await;
    }
}

#[async_trait]
pub trait RemoveController {
    async fn remove_controller(&self, controller: V2<usize>);
}

#[async_trait]
impl<T> RemoveController for T
where
    T: SendTerritory + Sync,
{
    async fn remove_controller(&self, controller: V2<usize>) {
        self.send_territory(move |territory| territory.remove_controller(&controller))
            .await;
    }
}

#[async_trait]
pub trait Controlled {
    async fn controlled(&self, controller: V2<usize>) -> HashSet<V2<usize>>;
}

#[async_trait]
impl<T> Controlled for T
where
    T: SendTerritory + Sync,
{
    async fn controlled(&self, controller: V2<usize>) -> HashSet<V2<usize>> {
        self.send_territory(move |territory| territory.controlled(&controller))
            .await
    }
}

#[async_trait]
pub trait SetControlDurations {
    async fn set_control_durations(
        &self,
        controller: V2<usize>,
        durations: HashMap<V2<usize>, Duration>,
        game_micros: u128,
    );
}

#[async_trait]
impl<T> SetControlDurations for T
where
    T: DrawWorld + Micros + SendTerritory + SendWorld + Sync,
{
    async fn set_control_durations(
        &self,
        controller: V2<usize>,
        durations: HashMap<V2<usize>, Duration>,
        game_micros: u128,
    ) {
        let changes = self
            .send_territory(move |territory| {
                territory.set_durations(controller, &durations, &game_micros)
            })
            .await;

        let when = self.micros().await;

        let affected: Vec<V2<usize>> = self
            .send_world(move |world| {
                changes
                    .iter()
                    .flat_map(|change| world.expand_position(&change.position))
                    .collect()
            })
            .await;

        for tile in affected {
            self.draw_world_tile(tile, when)
        }
    }
}

#[async_trait]
pub trait UpdateTerritory {
    async fn update_territory(&mut self, controller: V2<usize>);
}

#[async_trait]
impl<X> UpdateTerritory for X
where
    X: Micros
        + PathfinderWithoutPlannedRoads
        + SendParameters
        + SetControlDurations
        + Clone
        + Send
        + Sync,
{
    async fn update_territory(&mut self, controller: V2<usize>) {
        let duration = self
            .send_parameters(|parameters| parameters.town_travel_duration)
            .await;
        let corners = get_corners(&controller);
        let pathfinder = self.pathfinder_without_planned_roads();
        let durations = pathfinder.positions_within(corners, duration).await;
        let micros = self.micros().await;
        self.set_control_durations(controller, durations, micros)
            .await
    }
}

#[async_trait]
pub trait WhoControlsTile {
    async fn who_controls_tile(&self, tile: &V2<usize>) -> Option<V2<usize>>;
}

#[async_trait]
impl<T> WhoControlsTile for T
where
    T: SendTerritory + Sync,
{
    async fn who_controls_tile(&self, tile: &V2<usize>) -> Option<V2<usize>> {
        let tile = *tile;
        self.send_territory(move |territory| {
            territory
                .who_controls_tile(&tile)
                .map(|claim| claim.position)
        })
        .await
    }
}
