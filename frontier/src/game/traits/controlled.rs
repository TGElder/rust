use crate::game::Game;
use commons::V2;
use std::collections::HashSet;

pub trait Controlled {
    fn controlled(&self, position: &V2<usize>) -> HashSet<V2<usize>>;
}

impl Controlled for HashSet<V2<usize>> {
    fn controlled(&self, _: &V2<usize>) -> HashSet<V2<usize>> {
        self.clone()
    }
}

impl Controlled for Game {
    fn controlled(&self, position: &V2<usize>) -> HashSet<V2<usize>> {
        self.game_state.territory.controlled(&position)
    }
}
