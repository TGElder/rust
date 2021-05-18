use commons::async_trait::async_trait;

use crate::bridge::Bridge;
use crate::traits::{UpdateEdgesAllPathfinders, WithBridges};
use crate::travel_duration::EdgeDuration;

#[async_trait]
pub trait AddBridge {
    async fn add_bridge(&self, bridge: Bridge);
}

#[async_trait]
impl<T> AddBridge for T
where
    T: UpdateEdgesAllPathfinders + WithBridges + Sync,
{
    async fn add_bridge(&self, bridge: Bridge) {
        let edge_durations = bridge.edge_durations().collect::<Vec<_>>();
        self.mut_bridges(|bridges| bridges.insert(bridge.edge, bridge))
            .await;
        self.update_edges_all_pathfinders(edge_durations).await;
    }
}

#[async_trait]
pub trait RemoveBridge {
    async fn remove_bridge(&self, bridge: Bridge);
}

#[async_trait]
impl<T> RemoveBridge for T
where
    T: UpdateEdgesAllPathfinders + WithBridges + Sync,
{
    async fn remove_bridge(&self, bridge: Bridge) {
        let edge_durations = bridge
            .edge_durations()
            .map(|duration| EdgeDuration {
                duration: None,
                ..duration
            })
            .collect::<Vec<_>>();
        self.mut_bridges(|bridges| bridges.remove(&bridge.edge))
            .await;
        self.update_edges_all_pathfinders(edge_durations).await;
    }
}
