use crate::actors::UpdateRoads;
use commons::async_trait::async_trait;
use commons::edge::Edge;
use std::collections::HashSet;

#[async_trait]
pub trait BuildRoad {
    async fn add_road(&mut self, edge: &Edge);

    async fn remove_road(&mut self, edge: &Edge);
}

#[async_trait]
impl BuildRoad for UpdateRoads {
    async fn add_road(&mut self, edge: &Edge) {
        self.add_road(edge).await;
    }

    async fn remove_road(&mut self, edge: &Edge) {
        self.remove_road(edge).await;
    }
}

#[async_trait]
impl BuildRoad for HashSet<Edge> {
    async fn add_road(&mut self, edge: &Edge) {
        self.insert(*edge);
    }

    async fn remove_road(&mut self, edge: &Edge) {
        self.remove(edge);
    }
}
