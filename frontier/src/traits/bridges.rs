use commons::async_trait::async_trait;
use commons::edge::Edge;
use futures::FutureExt;

use crate::bridge::BridgeType::Built;
use crate::bridge::{Bridge, Bridges};
use crate::traits::{
    PathfinderForPlayer, PathfinderForRoutes, SendBridgeArtistActor, UpdatePathfinderEdges,
    WithBridges,
};

#[async_trait]
pub trait AddBridge {
    async fn add_bridge(&self, bridge: Bridge);
}

#[async_trait]
impl<T> AddBridge for T
where
    T: SendBridgeArtistActor + UpdateBridgesAllPathfinders + WithBridges + Sync,
{
    async fn add_bridge(&self, bridge: Bridge) {
        let edge = bridge.total_edge();

        let bridge_to_add = bridge.clone();
        self.mut_bridges(|bridges| bridges.entry(edge).or_default().insert(bridge_to_add))
            .await;

        self.update_bridges_all_pathfinders(&edge).await;

        self.send_bridge_artist_future_background(move |bridge_artist| {
            bridge_artist.draw_bridge(bridge).boxed()
        });
    }
}

#[async_trait]
pub trait RemoveBridge {
    async fn remove_bridge(&self, bridge: Bridge) -> bool;
}

#[async_trait]
impl<T> RemoveBridge for T
where
    T: SendBridgeArtistActor + UpdateBridgesAllPathfinders + WithBridges + Sync,
{
    async fn remove_bridge(&self, bridge: Bridge) -> bool {
        let edge = bridge.total_edge();

        let removed = self
            .mut_bridges(|bridges| {
                let bridges = bridges.get_mut(&edge);
                match bridges {
                    Some(bridges) => bridges.remove(&bridge),
                    None => false,
                }
            })
            .await;

        if !removed {
            return false;
        }

        self.update_bridges_all_pathfinders(&edge).await;

        self.send_bridge_artist_future_background(move |bridge_artist| {
            bridge_artist.erase_bridge(edge).boxed()
        });

        true
    }
}

#[async_trait]
pub trait UpdateBridgesAllPathfinders {
    async fn update_bridges_all_pathfinders(&self, edge: &Edge);
}

#[async_trait]
impl<T> UpdateBridgesAllPathfinders for T
where
    T: PathfinderForPlayer
        + PathfinderForRoutes
        + UpdatePathfinderEdges
        + WithBridges
        + Send
        + Sync,
{
    async fn update_bridges_all_pathfinders(&self, edge: &Edge) {
        let bridges = unwrap_or!(
            self.with_bridges(|bridges| bridges.get(edge).cloned())
                .await,
            return
        );

        let player_bridge = bridges
            .iter()
            .filter(|bridge| *bridge.bridge_type() == Built)
            .min_by_key(|bridge| bridge.total_duration());
        let route_bridge = bridges
            .iter()
            .min_by_key(|bridges| bridges.total_duration());

        let player_edge_durations = match player_bridge {
            Some(bridge) => bridge.total_edge_durations().collect(),
            None => vec![],
        };
        let route_edge_durations = match route_bridge {
            Some(bridge) => bridge.total_edge_durations().collect(),
            None => vec![],
        };

        let player_pathfinder = self.player_pathfinder();
        let route_pathfinder = self.routes_pathfinder();

        join!(
            self.update_pathfinder_edges(player_pathfinder, player_edge_durations),
            self.update_pathfinder_edges(route_pathfinder, route_edge_durations)
        );
    }
}

#[async_trait]
pub trait AllBridges {
    async fn all_bridges(&self) -> Bridges;
}

#[async_trait]
impl<T> AllBridges for T
where
    T: WithBridges + Sync,
{
    async fn all_bridges(&self) -> Bridges {
        self.with_bridges(|bridges| (*bridges).clone()).await
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
                .map(|(edge, bridge)| {
                    (
                        *edge,
                        bridge
                            .iter()
                            .filter(|bridge| *bridge.bridge_type() == Built)
                            .cloned()
                            .collect(),
                    )
                })
                .collect()
        })
        .await
    }
}
