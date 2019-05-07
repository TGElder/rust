use std::cmp::Ordering;

pub fn float_ordering(a: &f32, b: &f32) -> Ordering {
    a.partial_cmp(b).unwrap()
}
