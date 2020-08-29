use crate::game::Game;
use crate::world::WorldObject;
use commons::V2;
use std::collections::HashSet;

pub trait RemoveObject {
    fn remove_object(&mut self, position: &V2<usize>);
}

impl RemoveObject for HashSet<V2<usize>> {
    fn remove_object(&mut self, position: &V2<usize>) {
        self.insert(*position);
    }
}

impl RemoveObject for Game {
    fn remove_object(&mut self, position: &V2<usize>) {
        self.force_object(WorldObject::None, *position);
    }
}
