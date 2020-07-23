use crate::game::Game;
use commons::edge::Edge;
use std::collections::HashSet;

pub trait BuildRoad {
    fn add_road(&mut self, edge: &Edge);

    fn remove_road(&mut self, edge: &Edge);
}

impl BuildRoad for Game {
    fn add_road(&mut self, edge: &Edge) {
        self.game_state.world.set_road(edge, true)
    }

    fn remove_road(&mut self, edge: &Edge) {
        self.game_state.world.set_road(edge, false)
    }
}

impl BuildRoad for HashSet<Edge> {
    fn add_road(&mut self, edge: &Edge) {
        self.insert(*edge);
    }

    fn remove_road(&mut self, edge: &Edge) {
        self.remove(edge);
    }
}
