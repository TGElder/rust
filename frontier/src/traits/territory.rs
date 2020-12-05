use commons::async_trait::async_trait;
use commons::V2;

use crate::traits::SendTerritory;

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
