use crate::game::Game;
use crate::world::World;

pub trait HasWorld {
    fn world(&self) -> &World;
    fn world_mut(&mut self) -> &mut World;
}

impl HasWorld for World {
    fn world(&self) -> &World {
        self
    }

    fn world_mut(&mut self) -> &mut World {
        self
    }
}

impl HasWorld for Game {
    fn world(&self) -> &World {
        &self.game_state.world
    }

    fn world_mut(&mut self) -> &mut World {
        &mut self.game_state.world
    }
}
