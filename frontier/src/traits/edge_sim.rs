use std::collections::HashSet;

use commons::async_trait::async_trait;
use commons::edge::Edge;
use futures::FutureExt;

use super::SendEdgeBuildSim;

#[async_trait]
pub trait RefreshEdges {
    async fn refresh_edges(&self, edges: HashSet<Edge>);
}

#[async_trait]

impl<T> RefreshEdges for T
where
    T: SendEdgeBuildSim,
{
    async fn refresh_edges(&self, edges: HashSet<Edge>) {
        self.send_edge_build_sim_future(move |edge_sim| edge_sim.refresh_edges(edges).boxed())
            .await;
    }
}
