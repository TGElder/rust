use crate::game::Game;

pub trait Micros {
    fn micros(&self) -> &u128;
}

impl Micros for u128 {
    fn micros(&self) -> &u128 {
        &self
    }
}

impl Micros for Game {
    fn micros(&self) -> &u128 {
        &self.game_state.game_micros
    }
}
