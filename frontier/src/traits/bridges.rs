use std::collections::HashSet;

use commons::async_trait::async_trait;
use commons::edge::Edge;
use commons::grid::Grid;
use commons::V2;
use futures::FutureExt;

use crate::bridges::BridgeType::Built;
use crate::bridges::{Bridge, Bridges, BridgesExt};
use crate::traits::has::HasParameters;
use crate::traits::{
    DrawWorld, PathfinderForPlayer, PathfinderForRoutes, SendBridgeArtistActor,
    UpdatePathfinderEdges, WithBridges, WithWorld,
};
use crate::world::ROAD_WIDTH;

#[async_trait]
pub trait AddBridge {
    async fn add_bridge(&self, bridge: Bridge);
}

#[async_trait]
impl<T> AddBridge for T
where
    T: AddPlatforms + SendBridgeArtistActor + UpdateBridgesAllPathfinders + WithBridges + Sync,
{
    async fn add_bridge(&self, bridge: Bridge) {
        let edge = bridge.total_edge();

        let bridge_to_add = bridge.clone();
        let (added, platforms_to_add) = self
            .mut_bridges(|bridges| {
                let edge_bridges = bridges.entry(edge).or_default();

                if edge_bridges.contains(&bridge_to_add) {
                    return (false, hashset! {});
                }

                edge_bridges.insert(bridge_to_add);

                let mut platforms_to_add = HashSet::with_capacity(2);
                if bridges.count_platforms_at(edge.from(), &Built) == 1 {
                    platforms_to_add.insert(*edge.from());
                }
                if bridges.count_platforms_at(edge.to(), &Built) == 1 {
                    platforms_to_add.insert(*edge.to());
                }
                (true, platforms_to_add)
            })
            .await;

        if !added {
            return;
        }

        join!(
            self.update_bridges_all_pathfinders(&edge),
            self.add_platforms(platforms_to_add)
        );

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
    T: RemovePlatforms + SendBridgeArtistActor + UpdateBridgesAllPathfinders + WithBridges + Sync,
{
    async fn remove_bridge(&self, bridge: Bridge) -> bool {
        let edge = bridge.total_edge();

        let (removed, platforms_to_remove) = self
            .mut_bridges(|bridges| {
                let edge_bridges = bridges.get_mut(&edge);

                let removed = match edge_bridges {
                    Some(edge_bridges) => edge_bridges.remove(&bridge),
                    None => false,
                };

                if !removed {
                    return (false, hashset! {});
                }

                let mut platforms_to_remove = HashSet::with_capacity(2);
                if bridges.count_platforms_at(edge.from(), &Built) == 0 {
                    platforms_to_remove.insert(*edge.from());
                }
                if bridges.count_platforms_at(edge.to(), &Built) == 0 {
                    platforms_to_remove.insert(*edge.to());
                }
                (true, platforms_to_remove)
            })
            .await;

        if !removed {
            return false;
        }

        join!(
            self.update_bridges_all_pathfinders(&edge),
            self.remove_platforms(platforms_to_remove)
        );

        self.send_bridge_artist_future_background(move |bridge_artist| {
            bridge_artist.erase_bridge(bridge).boxed()
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
    T: HasParameters
        + PathfinderForPlayer
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

        let player_duration_fn = &self.parameters().player_bridge_duration_fn;
        let npc_duration_fn = &self.parameters().npc_bridge_duration_fn;

        let player_edge_durations = player_duration_fn
            .lowest_duration_bridge(&bridges)
            .map(|bridge| player_duration_fn.total_edge_durations(bridge))
            .map(|iterator| iterator.collect::<Vec<_>>())
            .unwrap_or_default();
        let route_edge_durations = npc_duration_fn
            .lowest_duration_bridge(&bridges)
            .map(|bridge| npc_duration_fn.total_edge_durations(bridge))
            .map(|iterator| iterator.collect::<Vec<_>>())
            .unwrap_or_default();

        let player_pathfinder = self.player_pathfinder();
        let routes_pathfinder = self.routes_pathfinder();

        join!(
            self.update_pathfinder_edges(player_pathfinder, player_edge_durations),
            self.update_pathfinder_edges(routes_pathfinder, route_edge_durations)
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
                            .filter(|bridge| bridge.bridge_type == Built)
                            .cloned()
                            .collect(),
                    )
                })
                .collect()
        })
        .await
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
        if positions.is_empty() {
            return;
        }
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
        if positions.is_empty() {
            return;
        }
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
