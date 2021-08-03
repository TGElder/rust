use commons::v2;

use crate::traits::has::HasParameters;
use crate::traits::{
    AllBridges, PathfinderForPlayer, PathfinderForRoutes, UpdatePathfinderEdges,
    UpdatePositionsAllPathfinders,
};

pub struct SetupPathfinders<T> {
    cx: T,
}

impl<T> SetupPathfinders<T>
where
    T: AllBridges
        + HasParameters
        + PathfinderForPlayer
        + PathfinderForRoutes
        + UpdatePathfinderEdges
        + UpdatePositionsAllPathfinders,
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
        let bridges = self.cx.all_bridges().await;

        let player_duration_fn = &self.cx.parameters().player_bridge_duration_fn;
        let player_edge_durations = bridges
            .values()
            .flat_map(|bridges| {
                bridges
                    .iter()
                    .min_by_key(|bridge| player_duration_fn.total_duration(bridge))
            })
            .flat_map(|bridge| player_duration_fn.total_edge_durations(bridge))
            .collect::<Vec<_>>();

        let npc_duration_fn = &self.cx.parameters().npc_bridge_duration_fn;
        let routes_edge_durations = bridges
            .values()
            .flat_map(|bridges| {
                bridges
                    .iter()
                    .min_by_key(|bridge| npc_duration_fn.total_duration(bridge))
            })
            .flat_map(|bridge| npc_duration_fn.total_edge_durations(bridge))
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
