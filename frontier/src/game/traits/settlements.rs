use crate::game::Game;
use crate::settlement::Settlement;
use commons::V2;
use std::collections::HashMap;

pub trait Settlements {
    fn settlements(&self) -> &HashMap<V2<usize>, Settlement>;
}

impl Settlements for HashMap<V2<usize>, Settlement> {
    fn settlements(&self) -> &HashMap<V2<usize>, Settlement> {
        &self
    }
}

impl Settlements for Game {
    fn settlements(&self) -> &HashMap<V2<usize>, Settlement> {
        &self.game_state.settlements
    }
}
