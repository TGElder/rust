use std::collections::HashMap;
use std::time::Duration;

use commons::async_trait::async_trait;
use commons::{Grid, V2};

use crate::traits::{DrawWorld, Micros, SendTerritory, SendWorld};

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
    async fn remove_controller(&self, controller: &V2<usize>);
}

#[async_trait]
impl<T> RemoveController for T
where
    T: SendTerritory + Sync,
{
    async fn remove_controller(&self, controller: &V2<usize>) {
        let controller = *controller;
        self.send_territory(move |territory| territory.remove_controller(&controller))
            .await;
    }
}

// pub trait SetTerritory {
//     fn set_territory(&mut self, states: Vec<TerritoryState>);
// }

// impl<T> SetTerritory
//     where T: SendTerritory + Sync
// {
//     fn set_territory(&mut self, mut states: Vec<TerritoryState>) {
//         let mut changes = vec![];
//         for TerritoryState {
//             controller,
//             durations,
//         } in states
//         {
//     }
// }

#[async_trait]
pub trait SetDurations {
    async fn set_control_durations(
        &self,
        controller: V2<usize>,
        durations: HashMap<V2<usize>, Duration>,
        game_micros: u128,
    );
}

#[async_trait]
impl<T> SetDurations for T
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
