use commons::V2;

pub trait InBounds {
    fn in_bounds(&self, position: &V2<usize>) -> bool;
}
