use crate::game::Game;
use crate::road_builder::RoadBuilderResult;
use commons::edge::Edge;
use std::collections::HashSet;

pub trait BuildRoad {
    fn add_road(&mut self, edge: &Edge);

    fn remove_road(&mut self, edge: &Edge);
}

impl BuildRoad for Game {
    fn add_road(&mut self, edge: &Edge) {
        let result = RoadBuilderResult::new(vec![*edge.from(), *edge.to()], false);
        self.update_roads(result);
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
