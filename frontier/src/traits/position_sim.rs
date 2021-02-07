use std::collections::HashSet;

use commons::V2;
use futures::FutureExt;

use super::SendPositionBuildSim;

pub trait RefreshPositions {
    fn refresh_positions(&self, positions: HashSet<V2<usize>>);
}

impl<T> RefreshPositions for T
where
    T: SendPositionBuildSim,
{
    fn refresh_positions(&self, positions: HashSet<V2<usize>>) {
        self.send_position_build_sim_future_background(move |position_sim| {
            position_sim.refresh_positions(positions).boxed()
        });
    }
}
