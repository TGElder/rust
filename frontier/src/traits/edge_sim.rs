use std::collections::HashSet;

use commons::edge::Edge;

use super::SendEdgeBuildSim;

pub trait RefreshEdges {
    fn refresh_edges(&self, edges: HashSet<Edge>);
}

impl<T> RefreshEdges for T
where
    T: SendEdgeBuildSim,
{
    fn refresh_edges(&self, edges: HashSet<Edge>) {
        self.send_edge_build_sim_background(move |edge_sim| edge_sim.refresh_edges(edges));
    }
}
