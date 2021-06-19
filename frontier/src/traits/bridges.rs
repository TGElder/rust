use commons::async_trait::async_trait;
use commons::edge::Edge;
use futures::FutureExt;

use crate::bridge::BridgeType::Built;
use crate::bridge::{Bridge, Bridges};
use crate::traits::{
    PathfinderForPlayer, PathfinderForRoutes, SendBridgeArtistActor, UpdateEdgesAllPathfinders,
    UpdatePathfinderEdges, WithBridges,
};
use crate::travel_duration::EdgeDuration;

#[async_trait]
pub trait AddBridge {
    async fn add_bridge(&self, bridge: Bridge);
}

#[async_trait]
impl<T> AddBridge for T
where
    T: PathfinderForPlayer
        + PathfinderForRoutes
        + SendBridgeArtistActor
        + UpdatePathfinderEdges
        + WithBridges
        + Sync,
{
    async fn add_bridge(&self, bridge: Bridge) {
        self.send_bridge_artist_future_background(move |bridge_artist| {
            bridge_artist.draw_bridge(bridge).boxed()
        });

        self.mut_bridges(|bridges| bridges.insert(bridge.edge, bridge))
            .await;

        let player_edge_durations = if bridge.bridge_type == Built {
            bridge.edge_durations().collect::<Vec<_>>()
        } else {
            vec![]
        };
        let routes_edge_durations = bridge.edge_durations().collect::<Vec<_>>();

        let player_pathfinder = self.player_pathfinder();
        let routes_pathfinder = self.routes_pathfinder();
        join!(
            async {
                if !player_edge_durations.is_empty() {
                    self.update_pathfinder_edges(player_pathfinder, player_edge_durations)
                        .await;
                }
            },
            async {
                if !routes_edge_durations.is_empty() {
                    self.update_pathfinder_edges(routes_pathfinder, routes_edge_durations)
                        .await;
                }
            }
        );
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

#[async_trait]
pub trait BuiltBridges {
    async fn built_bridges(&self) -> Bridges;
}

#[async_trait]
impl<T> BuiltBridges for T
where
    T: WithBridges + Sync,
{
    async fn built_bridges(&self) -> Bridges {
        self.with_bridges(|bridges| {
            bridges
                .iter()
                .filter(|(_, bridge)| bridge.bridge_type == Built)
                .map(|(edge, bridge)| (*edge, *bridge))
                .collect()
        })
        .await
    }
}
