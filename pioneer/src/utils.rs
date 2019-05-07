use std::cmp::Ordering;
use std::f64;

pub fn float_ordering(a: &&f64, b: &&f64) -> Ordering {
    a.partial_cmp(b).unwrap()
}
