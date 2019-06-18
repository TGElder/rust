extern crate nalgebra as na;
extern crate num;
extern crate serde;

pub mod edge;
pub mod index2d;
pub mod junction;
pub mod scale;

pub type M<T> = na::DMatrix<T>;
pub type V2<T> = na::Vector2<T>;
pub type V3<T> = na::Vector3<T>;

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

pub trait Grid<T> {
    fn in_bounds(&self, &V2<usize>) -> bool;
    fn get_cell_unsafe(&self, &V2<usize>) -> &T;
    fn mut_cell_unsafe(&mut self, &V2<usize>) -> &mut T;

    fn get_cell(&self, position: &V2<usize>) -> Option<&T> {
        if self.in_bounds(position) {
            Some(self.get_cell_unsafe(position))
        } else {
            None
        }
    }

    fn mut_cell(&mut self, position: &V2<usize>) -> Option<&mut T> {
        if self.in_bounds(position) {
            Some(self.mut_cell_unsafe(position))
        } else {
            None
        }
    }
}

impl<T> Grid<T> for M<T>
where
    T: 'static + Copy + Debug + PartialEq,
{
    fn in_bounds(&self, position: &V2<usize>) -> bool {
        let (width, height) = self.shape();
        position.x < width && position.y < height
    }

    fn get_cell_unsafe(&self, position: &V2<usize>) -> &T {
        &self[(position.x, position.y)]
    }

    fn mut_cell_unsafe(&mut self, position: &V2<usize>) -> &mut T {
        &mut self[(position.x, position.y)]
    }
}
