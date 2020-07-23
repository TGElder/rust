use crate::game::Game;

pub trait VisiblePositions {
    fn visible_land_positions(&self) -> usize;
}

impl VisiblePositions for Game {
    fn visible_land_positions(&self) -> usize {
        self.game_state.visible_land_positions
    }
}

impl VisiblePositions for usize {
    fn visible_land_positions(&self) -> usize {
        *self
    }
}
