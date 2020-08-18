use crate::game::Game;
use commons::V2;

pub trait RemoveSettlement {
    fn remove_settlement(&mut self, position: &V2<usize>);
}

impl RemoveSettlement for Vec<V2<usize>> {
    fn remove_settlement(&mut self, position: &V2<usize>) {
        self.push(*position);
    }
}

impl RemoveSettlement for Game {
    fn remove_settlement(&mut self, position: &V2<usize>) {
        self.remove_settlement(*position);
    }
}
