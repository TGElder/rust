use crate::game::Game;
use crate::settlement::Settlement;

pub trait AddSettlement {
    fn add_settlement(&mut self, settlement: Settlement);
}

impl AddSettlement for Vec<Settlement> {
    fn add_settlement(&mut self, settlement: Settlement) {
        self.push(settlement);
    }
}

impl AddSettlement for Game {
    fn add_settlement(&mut self, settlement: Settlement) {
        self.add_settlement(settlement);
    }
}
