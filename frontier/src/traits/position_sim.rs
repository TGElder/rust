use std::collections::HashSet;

use commons::async_trait::async_trait;
use commons::V2;
use futures::FutureExt;

use super::SendPositionBuildSim;

#[async_trait]
pub trait RefreshPositions {
    async fn refresh_positions(&self, positions: HashSet<V2<usize>>);
}

#[async_trait]
impl<T> RefreshPositions for T
where
    T: SendPositionBuildSim,
{
    async fn refresh_positions(&self, positions: HashSet<V2<usize>>) {
        self.send_position_build_sim_future(move |position_sim| {
            position_sim.refresh_positions(positions).boxed()
        })
        .await;
    }
}
