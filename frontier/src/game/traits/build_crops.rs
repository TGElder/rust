use crate::game::Game;
use crate::world::WorldObject;
use commons::V2;
use std::collections::HashMap;

pub trait BuildCrops {
    fn build_crops(&mut self, position: &V2<usize>, rotated: bool) -> bool;
}

impl BuildCrops for HashMap<V2<usize>, bool> {
    fn build_crops(&mut self, position: &V2<usize>, rotated: bool) -> bool {
        self.insert(*position, rotated);
        true
    }
}

impl BuildCrops for Game {
    fn build_crops(&mut self, position: &V2<usize>, rotated: bool) -> bool {
        self.add_object(WorldObject::Crop { rotated }, *position)
    }
}
