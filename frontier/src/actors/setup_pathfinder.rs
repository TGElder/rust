use commons::v2;

use crate::traits::has::HasParameters;
use crate::traits::{UpdateEdgesAllPathfinders, UpdatePositionsAllPathfinders, WithBridges};

pub struct SetupPathfinders<T> {
    cx: T,
}

impl<T> SetupPathfinders<T>
where
    T: HasParameters + UpdateEdgesAllPathfinders + UpdatePositionsAllPathfinders + WithBridges,
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

        let edge_durations = bridges
            .values()
            .flat_map(|bridge| bridge.edge_durations())
            .collect::<Vec<_>>();

        self.cx.update_edges_all_pathfinders(edge_durations).await;
    }
}
