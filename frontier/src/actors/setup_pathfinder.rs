use commons::v2;

use crate::bridge::BridgeType::Built;
use crate::traits::has::HasParameters;
use crate::traits::{
    PathfinderForPlayer, PathfinderForRoutes, UpdatePathfinderEdges, UpdatePositionsAllPathfinders,
    WithBridges,
};

pub struct SetupPathfinders<T> {
    cx: T,
}

impl<T> SetupPathfinders<T>
where
    T: HasParameters
        + PathfinderForPlayer
        + PathfinderForRoutes
        + UpdatePathfinderEdges
        + UpdatePositionsAllPathfinders
        + WithBridges,
{
    pub fn new(cx: T) -> SetupPathfinders<T> {
        SetupPathfinders { cx }
    }

    pub async fn init(&self) {
        join!(self.init_positions(), self.init_bridges());
    }

    async fn init_positions(&self) {
        let width = self.cx.parameters().width;

        let all_positions = (0..width).flat_map(move |x| (0..width).map(move |y| v2(x, y)));

        self.cx
            .update_positions_all_pathfinders(all_positions)
            .await;
    }

    async fn init_bridges(&self) {
        let bridges = self.cx.with_bridges(|bridges| (*bridges).clone()).await;

        let player_edge_durations = bridges
            .values()
            .filter(|bridge| bridge.bridge_type == Built)
            .flat_map(|bridge| bridge.edge_durations())
            .collect::<Vec<_>>();
        let routes_edge_durations = bridges
            .values()
            .flat_map(|bridge| bridge.edge_durations())
            .collect::<Vec<_>>();

        let player_pathfinder = self.cx.player_pathfinder();
        let routes_pathfinder = self.cx.routes_pathfinder();

        join!(
            self.cx
                .update_pathfinder_edges(player_pathfinder, player_edge_durations),
            self.cx
                .update_pathfinder_edges(routes_pathfinder, routes_edge_durations),
        );
    }
}
