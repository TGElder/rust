use std::collections::HashSet;

use commons::async_trait::async_trait;
use commons::edge::Edge;
use commons::V2;
use futures::FutureExt;

use crate::bridge::Bridge;
use crate::commons::grid::Grid;
use crate::traits::{
    DrawWorld, SendBridgeArtistActor, UpdateEdgesAllPathfinders, WithBridges, WithWorld,
};
use crate::travel_duration::EdgeDuration;
use crate::world::ROAD_WIDTH;

#[async_trait]
pub trait AddBridge {
    async fn add_bridge(&self, bridge: Bridge);
}

#[async_trait]
impl<T> AddBridge for T
where
    T: AddPlatforms + SendBridgeArtistActor + UpdateEdgesAllPathfinders + WithBridges + Sync,
{
    async fn add_bridge(&self, bridge: Bridge) {
        let bridge_to_insert = bridge.clone();
        self.mut_bridges(|bridges| bridges.insert(bridge_to_insert.edge, bridge_to_insert))
            .await;

        let edge_durations = bridge.edge_durations().collect::<Vec<_>>();
        self.update_edges_all_pathfinders(edge_durations).await;

        self.add_platforms(hashset! {*bridge.edge.from(), *bridge.edge.to()})
            .await;

        self.send_bridge_artist_future_background(|bridge_artist| {
            bridge_artist.draw_bridge(bridge).boxed()
        });
    }
}

#[async_trait]
pub trait RemoveBridge {
    async fn remove_bridge(&self, edge: Edge) -> bool;
}

#[async_trait]
impl<T> RemoveBridge for T
where
    T: RemovePlatforms + SendBridgeArtistActor + UpdateEdgesAllPathfinders + WithBridges + Sync,
{
    async fn remove_bridge(&self, edge: Edge) -> bool {
        let removed = self
            .with_bridges(|bridges| bridges.get(&edge).is_some())
            .await;
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

        self.remove_platforms(hashset! {*edge.from(), *edge.to()})
            .await;

        self.send_bridge_artist_future_background(move |bridge_artist| {
            bridge_artist.erase_bridge(edge).boxed()
        });

        true
    }
}

#[async_trait]
pub trait AddPlatforms {
    async fn add_platforms(&self, positions: HashSet<V2<usize>>);
}

#[async_trait]
impl<T> AddPlatforms for T
where
    T: DrawWorld + WithWorld + Sync,
{
    async fn add_platforms(&self, positions: HashSet<V2<usize>>) {
        self.mut_world(|world| {
            for position in positions.iter() {
                let cell = unwrap_or!(world.mut_cell(position), continue);
                cell.platform.horizontal.width = ROAD_WIDTH;
                cell.platform.vertical.width = ROAD_WIDTH;
            }
        })
        .await;
        self.draw_world_tiles(positions).await;
    }
}

#[async_trait]
pub trait RemovePlatforms {
    async fn remove_platforms(&self, positions: HashSet<V2<usize>>);
}

#[async_trait]
impl<T> RemovePlatforms for T
where
    T: DrawWorld + WithWorld + Sync,
{
    async fn remove_platforms(&self, positions: HashSet<V2<usize>>) {
        self.mut_world(|world| {
            for position in positions.iter() {
                let cell = unwrap_or!(world.mut_cell(position), continue);
                cell.platform.horizontal.width = 0.0;
                cell.platform.vertical.width = 0.0;
            }
        })
        .await;
        self.draw_world_tiles(positions).await;
    }
}
