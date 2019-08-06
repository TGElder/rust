pub extern crate image;
pub extern crate nalgebra as na;
pub extern crate num;
pub extern crate rand;
extern crate serde;

pub mod edge;
pub mod grid;
pub mod index2d;
pub mod junction;
pub mod scale;

pub type M<T> = na::DMatrix<T>;
pub type V2<T> = na::Vector2<T>;
pub type V3<T> = na::Vector3<T>;

use crate::scale::*;
pub use grid::*;
use num::Float;
use std::cmp::Ordering;
use std::fmt::Debug;

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

pub fn same_elements<T>(a: &Vec<T>, b: &Vec<T>) -> bool
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
    return true;
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
