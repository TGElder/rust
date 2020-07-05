use crate::game::Game;
use crate::settlement::Settlement;

pub trait AddSettlement {
    fn add_settlement(&mut self, settlement: Settlement) -> bool;
}

impl AddSettlement for Vec<Settlement> {
    fn add_settlement(&mut self, settlement: Settlement) -> bool {
        self.push(settlement);
        true
    }
}

impl AddSettlement for Game {
    fn add_settlement(&mut self, settlement: Settlement) -> bool {
        self.add_settlement(settlement)
    }
}
