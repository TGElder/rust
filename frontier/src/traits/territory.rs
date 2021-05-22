use std::collections::{HashMap, HashSet};
use std::time::Duration;

use commons::async_trait::async_trait;
use commons::grid::get_corners;
use commons::V2;

use crate::traits::has::HasParameters;
use crate::traits::{
    DrawWorld, ExpandPositions, Micros, PathfinderForRoutes, PositionsWithin, WithTerritory,
    WithWorld,
};
use crate::travel_duration::{land, TravelMode, TravelPosition};

#[async_trait]
pub trait AddController {
    async fn add_controller(&self, controller: V2<usize>);
}

#[async_trait]
impl<T> AddController for T
where
    T: WithTerritory + Sync,
{
    async fn add_controller(&self, controller: V2<usize>) {
        self.mut_territory(|territory| territory.add_controller(controller))
            .await;
    }
}

#[async_trait]
pub trait RemoveController {
    async fn remove_controller(&self, controller: &V2<usize>);
}

#[async_trait]
impl<T> RemoveController for T
where
    T: WithTerritory + Sync,
{
    async fn remove_controller(&self, controller: &V2<usize>) {
        self.mut_territory(|territory| territory.remove_controller(controller))
            .await;
    }
}

#[async_trait]
pub trait Controlled {
    async fn controlled(&self, position: &V2<usize>) -> HashSet<V2<usize>>;
}

#[async_trait]
impl<T> Controlled for T
where
    T: WithTerritory + Sync,
{
    async fn controlled(&self, position: &V2<usize>) -> HashSet<V2<usize>> {
        self.with_territory(|territory| territory.controlled(position))
            .await
    }
}

#[async_trait]
pub trait SetControlDurations {
    async fn set_control_durations(
        &self,
        controller: V2<usize>,
        durations: &HashMap<V2<usize>, Duration>,
        game_micros: &u128,
    );
}

#[async_trait]
impl<T> SetControlDurations for T
where
    T: DrawWorld + ExpandPositions + Micros + WithTerritory + WithWorld + Sync,
{
    async fn set_control_durations(
        &self,
        controller: V2<usize>,
        durations: &HashMap<V2<usize>, Duration>,
        game_micros: &u128,
    ) {
        let positions = self
            .mut_territory(move |territory| {
                territory.set_durations(controller, durations, game_micros)
            })
            .await
            .into_iter()
            .map(|change| change.position)
            .collect();

        let tiles = self.expand_positions(&positions).await;
        self.draw_world_tiles(tiles).await;
    }
}

#[async_trait]
pub trait UpdateTerritory {
    async fn update_territory(&self, controller: V2<usize>);
}

#[async_trait]
impl<T> UpdateTerritory for T
where
    T: HasParameters + Micros + PathfinderForRoutes + SetControlDurations + Clone + Send + Sync,
{
    async fn update_territory(&self, controller: V2<usize>) {
        let duration = self.parameters().town_travel_duration;
        let corners = get_land_corners(&controller);

        let pathfinder = self.routes_pathfinder();
        let durations = pathfinder.positions_within(&corners, &duration).await;
        let durations = get_land_durations(durations);

        let micros = self.micros().await;
        self.set_control_durations(controller, &durations, &micros)
            .await
    }
}

fn get_land_corners(position: &V2<usize>) -> Vec<TravelPosition> {
    get_corners(&position)
        .into_iter()
        .map(|corner| land(corner.x as u16, corner.y as u16))
        .collect()
}

fn get_land_durations(
    durations: HashMap<TravelPosition, Duration>,
) -> HashMap<V2<usize>, Duration> {
    durations
        .into_iter()
        .filter(|(position, _)| position.mode == TravelMode::Land)
        .map(|(position, duration)| (position.into(), duration))
        .collect()
}

#[async_trait]
pub trait WhoControlsTile {
    async fn who_controls_tile(&self, tile: &V2<usize>) -> Option<V2<usize>>;
}

#[async_trait]
impl<T> WhoControlsTile for T
where
    T: WithTerritory + Sync,
{
    async fn who_controls_tile(&self, tile: &V2<usize>) -> Option<V2<usize>> {
        self.with_territory(|territory| {
            territory
                .who_controls_tile(tile)
                .map(|claim| claim.position)
        })
        .await
    }
}

#[async_trait]
pub trait AnyoneControls {
    async fn anyone_controls(&self, position: &V2<usize>) -> bool;
}

#[async_trait]
impl<T> AnyoneControls for T
where
    T: WithTerritory + Sync,
{
    async fn anyone_controls(&self, position: &V2<usize>) -> bool {
        self.with_territory(|territory| territory.anyone_controls(position))
            .await
    }
}
