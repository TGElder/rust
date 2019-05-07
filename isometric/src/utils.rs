use std::cmp::Ordering;
use std::ffi::CString;
use V2;

#[allow(unused_mut)]
pub fn create_whitespace_cstring_with_len(length: usize) -> CString {
    let mut buffer: Vec<u8> = vec![b' '; length + 1];
    unsafe { CString::from_vec_unchecked(buffer) }
}

pub fn float_ordering(a: &&f32, b: &&f32) -> Ordering {
    a.partial_cmp(b).unwrap()
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Index2D {
    columns: usize,
    rows: usize,
}

#[derive(Debug, PartialEq)]
pub struct Index2DOutOfBounds {
    position: V2<usize>,
    index: Index2D,
}

impl Index2D {
    pub fn new(columns: usize, rows: usize) -> Index2D {
        Index2D { columns, rows }
    }

    pub fn get(&self, position: V2<usize>) -> Result<usize, Index2DOutOfBounds> {
        if position.x >= self.columns || position.y >= self.rows {
            Err(Index2DOutOfBounds {
                position,
                index: *self,
            })
        } else {
            Ok(position.y * self.columns + position.x)
        }
    }

    pub fn indices(&self) -> usize {
        self.columns * self.rows
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use v2;

    #[test]
    fn test_index_2d_indices() {
        let index = Index2D::new(4, 2);
        assert_eq!(index.indices(), 8);
    }

    #[test]
    fn test_index_2d_get_index() {
        let index = Index2D::new(4, 2);
        assert_eq!(index.get(v2(0, 0)).unwrap(), 0);
        assert_eq!(index.get(v2(1, 0)).unwrap(), 1);
        assert_eq!(index.get(v2(2, 0)).unwrap(), 2);
        assert_eq!(index.get(v2(3, 0)).unwrap(), 3);
        assert_eq!(index.get(v2(0, 1)).unwrap(), 4);
        assert_eq!(index.get(v2(1, 1)).unwrap(), 5);
        assert_eq!(index.get(v2(2, 1)).unwrap(), 6);
        assert_eq!(index.get(v2(3, 1)).unwrap(), 7);
    }

    #[test]
    fn test_index_2d_x_out_of_bounds() {
        let index = Index2D::new(4, 2);
        assert_eq!(
            index.get(v2(4, 0)),
            Err(Index2DOutOfBounds {
                position: v2(4, 0),
                index,
            })
        );
    }

    #[test]
    fn test_index_2d_y_out_of_bounds() {
        let index = Index2D::new(4, 2);
        assert_eq!(
            index.get(v2(0, 2)),
            Err(Index2DOutOfBounds {
                position: v2(0, 2),
                index,
            })
        );
    }
}
