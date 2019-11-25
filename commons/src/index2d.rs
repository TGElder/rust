use crate::{v2, V2};
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
}
