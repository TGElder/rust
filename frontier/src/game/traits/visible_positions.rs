use crate::game::Game;

pub trait VisiblePositions {
    fn total_visible_positions(&self) -> usize;
}

impl VisiblePositions for Game {
    fn total_visible_positions(&self) -> usize {
        self.game_state.total_visible_positions
    }
}

impl VisiblePositions for usize {
    fn total_visible_positions(&self) -> usize {
        *self
    }
}
