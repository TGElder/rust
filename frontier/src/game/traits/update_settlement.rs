use crate::game::Game;
use crate::settlement::Settlement;
use commons::V2;
use std::collections::HashMap;

pub trait UpdateSettlement {
    fn update_settlement(&mut self, settlement: Settlement);
}

impl UpdateSettlement for HashMap<V2<usize>, Settlement> {
    fn update_settlement(&mut self, settlement: Settlement) {
        self.insert(settlement.position, settlement);
    }
}

impl UpdateSettlement for Game {
    fn update_settlement(&mut self, settlement: Settlement) {
        self.update_settlement(settlement)
    }
}
