use std::collections::{HashMap, HashSet};
use std::time::Duration;

use commons::async_trait::async_trait;
use commons::grid::get_corners;
use commons::V2;

use crate::traits::has::HasParameters;
use crate::traits::{
    DrawWorld, ExpandPositions, Micros, PathfinderWithoutPlannedRoads, PositionsWithin,
    SendTerritory, SendWorld,
};

#[async_trait]
pub trait AddController {
    async fn add_controller(&self, controller: V2<usize>);
}

#[async_trait]
impl<T> AddController for T
where
    T: SendTerritory + Sync,
{
    async fn add_controller(&self, controller: V2<usize>) {
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
    T: DrawWorld + ExpandPositions + Micros + SendTerritory + SendWorld + Sync,
{
    async fn set_control_durations(
        &self,
        controller: V2<usize>,
        durations: HashMap<V2<usize>, Duration>,
        game_micros: u128,
    ) {
        let positions = self
            .send_territory(move |territory| {
                territory.set_durations(controller, &durations, &game_micros)
            })
            .await
            .into_iter()
            .map(|change| change.position)
            .collect();

        let when = self.micros().await;

        for tile in self.expand_positions(positions).await {
            self.draw_world_tile(tile, when)
        }
    }
}

#[async_trait]
pub trait UpdateTerritory {
    async fn update_territory(&self, controller: V2<usize>);
}

#[async_trait]
impl<T> UpdateTerritory for T
where
    T: HasParameters
        + Micros
        + PathfinderWithoutPlannedRoads
        + SetControlDurations
        + Clone
        + Send
        + Sync,
{
    async fn update_territory(&self, controller: V2<usize>) {
        let duration = self.parameters().town_travel_duration;
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
    async fn who_controls_tile(&self, tile: V2<usize>) -> Option<V2<usize>>;
}

#[async_trait]
impl<T> WhoControlsTile for T
where
    T: SendTerritory + Sync,
{
    async fn who_controls_tile(&self, tile: V2<usize>) -> Option<V2<usize>> {
        self.send_territory(move |territory| {
            territory
                .who_controls_tile(&tile)
                .map(|claim| claim.position)
        })
        .await
    }
}

#[async_trait]
pub trait AnyoneControls {
    async fn anyone_controls(&self, position: V2<usize>) -> bool;
}

#[async_trait]
impl<T> AnyoneControls for T
where
    T: SendTerritory + Sync,
{
    async fn anyone_controls(&self, position: V2<usize>) -> bool {
        self.send_territory(move |territory| territory.anyone_controls(&position))
            .await
    }
}
