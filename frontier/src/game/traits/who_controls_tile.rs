use crate::game::Game;
use commons::V2;

pub trait WhoControlsTile {
    fn who_controls_tile(&self, position: &V2<usize>) -> Option<&V2<usize>>;
}

impl WhoControlsTile for Option<&V2<usize>> {
    fn who_controls_tile(&self, _: &V2<usize>) -> Option<&V2<usize>> {
        *self
    }
}

impl WhoControlsTile for Game {
    fn who_controls_tile(&self, position: &V2<usize>) -> Option<&V2<usize>> {
        self.game_state
            .territory
            .who_controls_tile(position)
            .map(|claim| &claim.position)
    }
}
