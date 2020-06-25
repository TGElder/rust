use crate::game::Game;
use crate::world::World;

pub trait HasWorld {
    fn world(&self) -> &World;
}

impl HasWorld for World {
    fn world(&self) -> &World {
        &self
    }
}

impl HasWorld for Game {
    fn world(&self) -> &World {
        &self.game_state.world
    }
}
