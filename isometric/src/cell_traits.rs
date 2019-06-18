use commons::junction::*;
use commons::{v2, V2};

pub trait WithPosition {
    fn position(&self) -> V2<usize>;

    fn get_float_position(&self) -> V2<f32> {
        let position = self.position();
        v2(position.x as f32, position.y as f32)
    }
}

pub trait WithElevation {
    fn elevation(&self) -> f32;
}

pub trait WithVisibility {
    fn is_visible(&self) -> bool;
}

pub trait WithJunction {
    fn junction(&self) -> Junction;
}

impl WithPosition for PositionJunction {
    fn position(&self) -> V2<usize> {
        self.position
    }
}

impl WithJunction for PositionJunction {
    fn junction(&self) -> Junction {
        self.junction
    }
}
