use crate::grid::*;
use crate::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Clone, Copy, Deserialize, Serialize)]
pub struct Index2D {
    columns: usize,
    rows: usize,
}

#[derive(Debug, PartialEq)]
pub struct PositionOutOfBounds {
    position: V2<usize>,
    index_2d: Index2D,
}

#[derive(Debug, PartialEq)]
pub struct IndexOutOfBounds {
    index: usize,
    index_2d: Index2D,
}

impl Index2D {
    pub fn new(columns: usize, rows: usize) -> Index2D {
        Index2D { columns, rows }
    }

    pub fn get_index(&self, position: &V2<usize>) -> Result<usize, PositionOutOfBounds> {
        if position.x >= self.columns || position.y >= self.rows {
            Err(PositionOutOfBounds {
                position: *position,
                index_2d: *self,
            })
        } else {
            Ok(position.y * self.columns + position.x)
        }
    }

    pub fn get_position(&self, index: usize) -> Result<V2<usize>, IndexOutOfBounds> {
        if index >= self.indices() {
            Err(IndexOutOfBounds {
                index,
                index_2d: *self,
            })
        } else {
            Ok(v2(index % self.columns, index / self.columns))
        }
    }

    pub fn indices(&self) -> usize {
        self.columns * self.rows
    }
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
pub struct Vec2D<T> {
    index: Index2D,
    vector: Vec<T>,
}

impl<T> Vec2D<T>
where
    T: Clone,
{
    pub fn new(columns: usize, rows: usize, element: T) -> Vec2D<T> {
        Vec2D {
            index: Index2D::new(columns, rows),
            vector: vec![element; columns * rows],
        }
    }

    pub fn same_size_as<U>(grid: &dyn Grid<U>, element: T) -> Vec2D<T> {
        Vec2D::new(grid.width(), grid.height(), element)
    }

    pub fn get(&self, position: &V2<usize>) -> Result<&T, PositionOutOfBounds> {
        self.index
            .get_index(position)
            .map(|index| self.vector.get(index).unwrap())
    }

    pub fn get_mut(&mut self, position: &V2<usize>) -> Result<&mut T, PositionOutOfBounds> {
        self.index
            .get_index(position)
            .map(move |index| self.vector.get_mut(index).unwrap())
    }

    pub fn set(&mut self, position: &V2<usize>, value: T) -> Result<(), PositionOutOfBounds> {
        self.index
            .get_index(position)
            .map(|index| self.vector[index] = value)
    }
}

impl<T> Grid<T> for Vec2D<T>
where
    T: Clone,
{
    fn width(&self) -> usize {
        self.index.columns
    }

    fn height(&self) -> usize {
        self.index.rows
    }

    fn in_bounds(&self, position: &V2<usize>) -> bool {
        position.x < self.width() && position.y < self.height()
    }

    fn get_cell_unsafe(&self, position: &V2<usize>) -> &T {
        self.get(position).unwrap()
    }

    fn mut_cell_unsafe(&mut self, position: &V2<usize>) -> &mut T {
        self.get_mut(position).unwrap()
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_index_2d_indices() {
        let index = Index2D::new(4, 2);
        assert_eq!(index.indices(), 8);
    }

    #[test]
    fn test_index_2d_get_index() {
        let index = Index2D::new(4, 2);
        assert_eq!(index.get_index(&v2(0, 0)).unwrap(), 0);
        assert_eq!(index.get_index(&v2(1, 0)).unwrap(), 1);
        assert_eq!(index.get_index(&v2(2, 0)).unwrap(), 2);
        assert_eq!(index.get_index(&v2(3, 0)).unwrap(), 3);
        assert_eq!(index.get_index(&v2(0, 1)).unwrap(), 4);
        assert_eq!(index.get_index(&v2(1, 1)).unwrap(), 5);
        assert_eq!(index.get_index(&v2(2, 1)).unwrap(), 6);
        assert_eq!(index.get_index(&v2(3, 1)).unwrap(), 7);
    }

    #[test]
    fn test_index_2d_get_position() {
        let index = Index2D::new(4, 2);
        assert_eq!(index.get_position(0).unwrap(), v2(0, 0));
        assert_eq!(index.get_position(1).unwrap(), v2(1, 0));
        assert_eq!(index.get_position(2).unwrap(), v2(2, 0));
        assert_eq!(index.get_position(3).unwrap(), v2(3, 0));
        assert_eq!(index.get_position(4).unwrap(), v2(0, 1));
        assert_eq!(index.get_position(5).unwrap(), v2(1, 1));
        assert_eq!(index.get_position(6).unwrap(), v2(2, 1));
        assert_eq!(index.get_position(7).unwrap(), v2(3, 1));
    }

    #[test]
    fn test_index_2d_x_out_of_bounds() {
        let index_2d = Index2D::new(4, 2);
        assert_eq!(
            index_2d.get_index(&v2(4, 0)),
            Err(PositionOutOfBounds {
                position: v2(4, 0),
                index_2d,
            })
        );
    }

    #[test]
    fn test_index_2d_y_out_of_bounds() {
        let index_2d = Index2D::new(4, 2);
        assert_eq!(
            index_2d.get_index(&v2(0, 2)),
            Err(PositionOutOfBounds {
                position: v2(0, 2),
                index_2d,
            })
        );
    }

    #[test]
    fn test_index_2d_index_out_of_bounds() {
        let index_2d = Index2D::new(4, 2);
        assert_eq!(
            index_2d.get_position(8),
            Err(IndexOutOfBounds { index: 8, index_2d })
        );
    }

    #[test]
    fn test_vec_2d_same_size_as() {
        let vec_2d = Vec2D::same_size_as::<u8>(&M::zeros(5, 3), 0);
        assert_eq!(vec_2d.index.columns, 5);
        assert_eq!(vec_2d.index.rows, 3);
    }

    #[test]
    fn test_vec_2d_get_in_bounds() {
        let vec_2d = Vec2D::new(3, 2, 0);
        assert_eq!(vec_2d.get(&v2(0, 0)), Ok(&0));
    }

    #[test]
    fn test_vec_2d_get_out_of_bounds() {
        let vec_2d = Vec2D::new(3, 2, 0);
        assert_eq!(
            vec_2d.get(&v2(3, 0)),
            Err(PositionOutOfBounds {
                position: v2(3, 0),
                index_2d: vec_2d.index,
            })
        );
    }

    #[test]
    fn test_vec_2d_get_mut_in_bounds() {
        let mut vec_2d = Vec2D::new(3, 2, 0);
        if let Ok(value) = vec_2d.get_mut(&v2(0, 0)) {
            *value = 1;
        }
        assert_eq!(vec_2d.get(&v2(0, 0)), Ok(&1));
    }

    #[test]
    fn test_vec_2d_get_mut_out_of_bounds() {
        let mut vec_2d = Vec2D::new(3, 2, 0);
        let index_2d = vec_2d.index;
        assert_eq!(
            vec_2d.get_mut(&v2(3, 0)),
            Err(PositionOutOfBounds {
                position: v2(3, 0),
                index_2d,
            })
        );
    }

    #[test]
    fn test_vec_2d_set_in_bounds() {
        let mut vec_2d = Vec2D::new(3, 2, 0);
        vec_2d.set(&v2(0, 0), 1).unwrap();
        assert_eq!(vec_2d.get(&v2(0, 0)), Ok(&1));
    }

    #[test]
    fn test_vec_2d_set_out_of_bounds() {
        let mut vec_2d = Vec2D::new(3, 2, 0);
        assert_eq!(
            vec_2d.set(&v2(3, 0), 1),
            Err(PositionOutOfBounds {
                position: v2(3, 0),
                index_2d: vec_2d.index,
            })
        );
    }
}
