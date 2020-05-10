pub extern crate float_cmp;
pub extern crate futures;
pub extern crate image;
pub extern crate nalgebra as na;
pub extern crate num;
pub extern crate rand;
extern crate serde;

pub mod barycentric;
pub mod edge;
pub mod grid;
pub mod hub;
pub mod index2d;
pub mod junction;
pub mod scale;
pub mod update;

pub type M<T> = na::DMatrix<T>;
pub type V2<T> = na::Vector2<T>;
pub type V3<T> = na::Vector3<T>;

pub use grid::*;

use crate::scale::*;
use float_cmp::approx_eq;
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

pub trait Almost {
    fn almost(&self, other: &Self) -> bool;
}

impl Almost for f32 {
    fn almost(&self, other: &f32) -> bool {
        approx_eq!(f32, *self, *other, ulps = 5)
    }
}

impl Almost for f64 {
    fn almost(&self, other: &f64) -> bool {
        approx_eq!(f64, *self, *other, ulps = 5)
    }
}

impl<T> Almost for Option<T>
where
    T: Almost,
{
    fn almost(&self, other: &Option<T>) -> bool {
        match (self, other) {
            (Some(a), Some(b)) => a.almost(&b),
            (None, None) => true,
            _ => false,
        }
    }
}

impl<T> Almost for [T]
where
    T: Almost + Copy,
{
    fn almost(&self, other: &[T]) -> bool {
        self.iter()
            .enumerate()
            .all(|(i, value)| value.almost(&other[i]))
    }
}

impl<T> Almost for V2<T>
where
    T: Almost + Copy + Debug + PartialEq + 'static,
{
    fn almost(&self, other: &V2<T>) -> bool {
        self.iter()
            .enumerate()
            .all(|(i, value)| value.almost(&other[i]))
    }
}

impl<T> Almost for V3<T>
where
    T: Almost + Copy + Debug + PartialEq + 'static,
{
    fn almost(&self, other: &V3<T>) -> bool {
        self.iter()
            .enumerate()
            .all(|(i, value)| value.almost(&other[i]))
    }
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

    #[test]
    fn test_almost_option_both_some() {
        assert!(Some(0.3).almost(&Some(0.3)));
    }

    #[test]
    fn test_almost_option_lhs_none() {
        assert!(!None.almost(&Some(0.3)));
    }

    #[test]
    fn test_almost_option_rhs_none() {
        assert!(!Some(0.3).almost(&None));
    }

    #[test]
    fn test_almost_option_both_none() {
        assert!(None::<f32>.almost(&None::<f32>));
    }

    #[test]
    fn test_almost_vector() {
        assert!(vec![0.1, 0.2, 0.3].almost(&[0.1, 0.2, 0.3]));
    }

    #[test]
    fn test_almost_v2() {
        assert!(v2(0.1, 0.2).almost(&v2(0.1, 0.2)));
    }

    #[test]
    fn test_almost_v3() {
        assert!(v3(0.1, 0.2, 0.3).almost(&v3(0.1, 0.2, 0.3)));
    }
}
