use commons::async_trait::async_trait;
use commons::edge::Edge;
use futures::FutureExt;

use crate::bridge::Bridge;
use crate::traits::{SendBridgeArtistActor, UpdateEdgesAllPathfinders, WithBridges};
use crate::travel_duration::EdgeDuration;

#[async_trait]
pub trait AddBridge {
    async fn add_bridge(&self, bridge: Bridge);
}

#[async_trait]
impl<T> AddBridge for T
where
    T: SendBridgeArtistActor + UpdateEdgesAllPathfinders + WithBridges + Sync,
{
    async fn add_bridge(&self, bridge: Bridge) {
        let edge_durations = bridge.edge_durations().collect::<Vec<_>>();

        let bridge_for_artist = bridge.clone();
        self.send_bridge_artist_future_background(|bridge_artist| {
            bridge_artist.draw_bridge(bridge_for_artist).boxed()
        });

        self.mut_bridges(|bridges| bridges.insert(bridge.edge(), bridge))
            .await;

        self.update_edges_all_pathfinders(edge_durations).await;
    }
}

#[async_trait]
pub trait RemoveBridge {
    async fn remove_bridge(&self, edge: Edge) -> bool;
}

#[async_trait]
impl<T> RemoveBridge for T
where
    T: SendBridgeArtistActor + UpdateEdgesAllPathfinders + WithBridges + Sync,
{
    async fn remove_bridge(&self, edge: Edge) -> bool {
        let removed = self
            .mut_bridges(|bridges| bridges.remove(&edge))
            .await
            .is_some();

        if !removed {
            return false;
        }

        let edge_durations = vec![
            EdgeDuration {
                from: *edge.from(),
                to: *edge.to(),
                duration: None,
            },
            EdgeDuration {
                from: *edge.to(),
                to: *edge.from(),
                duration: None,
            },
        ];
        self.update_edges_all_pathfinders(edge_durations).await;

        self.send_bridge_artist_future_background(move |bridge_artist| {
            bridge_artist.erase_bridge(edge).boxed()
        });

        true
    }
}
