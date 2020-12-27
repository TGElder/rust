pub extern crate async_channel;
pub extern crate async_trait;
pub extern crate chrono;
pub extern crate float_cmp;
pub extern crate futures;
pub extern crate image;
pub extern crate log;
extern crate maplit;
pub extern crate nalgebra as na;
extern crate noise;
pub extern crate num;
pub extern crate rand;
extern crate serde;

pub mod almost;
pub mod barycentric;
pub mod edge;
pub mod equalize;
pub mod fn_sender;
pub mod grid;
pub mod hub;
pub mod index2d;
pub mod junction;
pub mod manhattan;
pub mod perlin;
pub mod rectangle;
pub mod scale;
mod unwrap_or;

pub type M<T> = na::DMatrix<T>;
pub type V2<T> = na::Vector2<T>;
pub type V3<T> = na::Vector3<T>;

pub use maplit::{btreemap, btreeset, hashmap, hashset};

use crate::scale::*;
use num::Float;
use std::cmp::Ordering;
use std::default::Default;
use std::fmt::Debug;
use std::sync::{Arc, Mutex};

pub type Arm<T> = Arc<Mutex<T>>;

pub fn v2<T: 'static + Copy + PartialEq + Debug>(x: T, y: T) -> na::Vector2<T> {
    na::Vector2::new(x, y)
}

pub fn v3<T: 'static + Copy + PartialEq + Debug>(x: T, y: T, z: T) -> na::Vector3<T> {
    na::Vector3::new(x, y, z)
}

pub fn unsafe_ordering<T: PartialOrd>(a: &T, b: &T) -> Ordering {
    a.partial_cmp(b).unwrap()
}

pub fn rescale<T>(matrix: M<T>, out_range: (T, T)) -> M<T>
where
    T: 'static + Float + Debug,
{
    let min = matrix.iter().min_by(unsafe_ordering).unwrap();
    let max = matrix.iter().max_by(unsafe_ordering).unwrap();
    let scale = Scale::new((*min, *max), out_range);
    matrix.map(|v| scale.scale(v))
}

pub fn same_elements<T>(a: &[T], b: &[T]) -> bool
where
    T: PartialEq,
{
    if a.len() != b.len() {
        return false;
    }
    for element in a {
        if !b.contains(&element) {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_rescale() {
        let matrix = M::from_vec(8, 1, vec![1.0, 2.0, 5.0, 1.0, 4.0, 2.0, 3.0, 2.0]);
        assert_eq!(
            rescale(matrix, (0.0, 1.0)),
            M::from_vec(8, 1, vec![0.0, 0.25, 1.0, 0.0, 0.75, 0.25, 0.5, 0.25])
        );
    }

    #[test]
    fn test_same_elements() {
        let a = vec![1, 2, 3];
        let b = vec![3, 2, 1];
        assert!(same_elements(&a, &b));
    }

    #[test]
    fn test_different_elements() {
        let a = vec![1, 2, 3];
        let b = vec![3, 4, 1];
        assert!(!same_elements(&a, &b));
    }
}
