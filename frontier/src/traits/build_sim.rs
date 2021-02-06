use std::collections::HashSet;

use commons::edge::Edge;
use commons::V2;

use super::SendBuildSim;

pub trait RefreshBuildSim {
    fn refresh_edges(&self, edges: HashSet<Edge>);
    fn refresh_positions(&self, positions: HashSet<V2<usize>>);
}

impl<T> RefreshBuildSim for T
where
    T: SendBuildSim,
{
    fn refresh_edges(&self, edges: HashSet<Edge>) {
        self.send_build_sim_background(move |build_sim| build_sim.refresh_edges(edges));
    }

    fn refresh_positions(&self, positions: HashSet<V2<usize>>) {
        self.send_build_sim_background(move |build_sim| build_sim.refresh_positions(positions));
    }
}
