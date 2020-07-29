use crate::world::World;
use commons::edge::Edge;

pub trait UpdateEdge {
    fn update_edge(&mut self, world: &World, edge: &Edge);
}

impl UpdateEdge for Vec<Edge> {
    fn update_edge(&mut self, _: &World, edge: &Edge) {
        self.push(*edge);
    }
}
