use super::*;

pub fn get_corners(position: &V2<usize>) -> Vec<V2<usize>> {
    vec![
        *position,
        v2(position.x + 1, position.y),
        v2(position.x + 1, position.y + 1),
        v2(position.x, position.y + 1),
    ]
}

pub fn get_corners_in_bounds(
    position: &V2<usize>,
    width: &usize,
    height: &usize,
) -> Vec<V2<usize>> {
    get_corners(position)
        .into_iter()
        .filter(|corner| corner.x < *width && corner.y < *height)
        .collect::<Vec<_>>()
}

pub trait Grid<T> {
    fn width(&self) -> usize;
    fn height(&self) -> usize;
    fn in_bounds(&self, position: &V2<usize>) -> bool;
    fn get_cell_unsafe(&self, position: &V2<usize>) -> &T;
    fn mut_cell_unsafe(&mut self, position: &V2<usize>) -> &mut T;

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

    fn is_corner_cell(&self, position: &V2<usize>) -> bool {
        position.x == 0 && position.y == 0
            || position.x == self.width() - 1 && position.y == 0
            || position.x == 0 && position.y == self.height() - 1
            || position.x == self.width() - 1 && position.y == self.height() - 1
    }

    fn is_edge_cell(&self, position: &V2<usize>) -> bool {
        position.x == 0
            || position.y == 0
            || position.x == self.width() - 1
            || position.y == self.height() - 1
    }

    fn edge_cells(&self) -> Vec<V2<usize>> {
        let mut out = vec![];
        for x in 0..self.width() {
            for y in 0..self.height() {
                let position = v2(x, y);
                if self.is_edge_cell(&position) {
                    out.push(position);
                }
            }
        }
        out
    }

    fn offset(&self, position: &V2<usize>, offset: V2<i32>) -> Option<V2<usize>> {
        let position_i32 = v2(position.x as i32, position.y as i32);
        let offset_i32 = position_i32 + offset;
        if offset_i32.x < 0 || offset_i32.y < 0 {
            return None;
        }
        let offset = v2(offset_i32.x as usize, offset_i32.y as usize);
        if self.in_bounds(&offset) {
            Some(offset)
        } else {
            None
        }
    }

    fn neighbours(&self, position: &V2<usize>) -> Vec<V2<usize>> {
        [v2(-1, 0), v2(0, -1), v2(1, 0), v2(0, 1)]
            .iter()
            .flat_map(|offset| self.offset(position, *offset))
            .collect()
    }

    fn get_corners_in_bounds(&self, position: &V2<usize>) -> Vec<V2<usize>> {
        get_corners(position)
            .into_iter()
            .filter(|position| self.in_bounds(position))
            .collect()
    }

    fn get_adjacent_tiles_in_bounds(&self, position: &V2<usize>) -> Vec<V2<usize>> {
        [v2(0, 0), v2(-1, 0), v2(-1, -1), v2(0, -1)]
            .iter()
            .flat_map(|delta| self.offset(position, *delta))
            .collect()
    }

    fn expand_position(&self, position: &V2<usize>) -> Vec<V2<usize>> {
        let mut out = vec![];
        let fx = if position.x == 0 { 0 } else { position.x - 1 };
        let fy = if position.y == 0 { 0 } else { position.y - 1 };
        for x in fx..position.x + 2 {
            for y in fy..position.y + 2 {
                let position = v2(x, y);
                if self.in_bounds(&position) {
                    out.push(position);
                }
            }
        }
        out
    }
}

pub fn extract_matrix<T, O>(grid: &dyn Grid<T>, function: &dyn Fn(&T) -> O) -> M<O>
where
    O: 'static + Copy + Debug + Default + PartialEq,
{
    let mut out = M::from_element(grid.width(), grid.height(), O::default());
    for x in 0..grid.width() {
        for y in 0..grid.height() {
            out[(x, y)] = function(grid.get_cell_unsafe(&v2(x, y)));
        }
    }
    out
}

impl<T> Grid<T> for M<T>
where
    T: 'static + Copy + Debug + PartialEq,
{
    fn width(&self) -> usize {
        self.shape().0
    }

    fn height(&self) -> usize {
        self.shape().1
    }

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
    fn corner_cell() {
        let matrix: M<u8> = M::zeros(3, 3);
        assert_eq!(matrix.is_corner_cell(&v2(0, 0)), true);
        assert_eq!(matrix.is_corner_cell(&v2(1, 0)), false);
        assert_eq!(matrix.is_corner_cell(&v2(2, 0)), true);
        assert_eq!(matrix.is_corner_cell(&v2(0, 1)), false);
        assert_eq!(matrix.is_corner_cell(&v2(1, 1)), false);
        assert_eq!(matrix.is_corner_cell(&v2(2, 1)), false);
        assert_eq!(matrix.is_corner_cell(&v2(0, 2)), true);
        assert_eq!(matrix.is_corner_cell(&v2(1, 2)), false);
        assert_eq!(matrix.is_corner_cell(&v2(2, 2)), true);
    }

    #[test]
    fn edges_cells_1x1() {
        let matrix: M<u8> = M::zeros(1, 1);
        let edge_cells = matrix.edge_cells();
        assert_eq!(edge_cells, vec![v2(0, 0)]);
    }

    #[test]
    fn edges_cells_2x1() {
        let matrix: M<u8> = M::zeros(2, 1);
        let edge_cells = matrix.edge_cells();
        assert_eq!(edge_cells.len(), 2);
        assert!(edge_cells.contains(&v2(0, 0)));
        assert!(edge_cells.contains(&v2(1, 0)));
    }

    #[test]
    fn edges_cells_1x2() {
        let matrix: M<u8> = M::zeros(1, 2);
        let edge_cells = matrix.edge_cells();
        assert_eq!(edge_cells.len(), 2);
        assert!(edge_cells.contains(&v2(0, 0)));
        assert!(edge_cells.contains(&v2(0, 1)));
    }

    #[test]
    fn edges_cells_2x2() {
        let matrix: M<usize> = M::zeros(2, 2);
        let edge_cells = matrix.edge_cells();
        assert_eq!(edge_cells.len(), 4);
        assert!(edge_cells.contains(&v2(0, 0)));
        assert!(edge_cells.contains(&v2(1, 0)));
        assert!(edge_cells.contains(&v2(0, 1)));
        assert!(edge_cells.contains(&v2(1, 1)));
    }

    #[test]
    fn edges_cells_3x3() {
        let matrix: M<usize> = M::zeros(3, 3);
        let edge_cells = matrix.edge_cells();
        assert_eq!(edge_cells.len(), 8);
        assert!(edge_cells.contains(&v2(0, 0)));
        assert!(edge_cells.contains(&v2(1, 0)));
        assert!(edge_cells.contains(&v2(2, 0)));
        assert!(edge_cells.contains(&v2(0, 1)));
        assert!(edge_cells.contains(&v2(2, 1)));
        assert!(edge_cells.contains(&v2(0, 2)));
        assert!(edge_cells.contains(&v2(1, 2)));
        assert!(edge_cells.contains(&v2(2, 2)));
    }

    #[test]
    fn offset_in_bounds() {
        let matrix = M::from_element(3, 3, 1.0);
        assert_eq!(matrix.offset(&v2(1, 1), v2(-1, -1)), Some(v2(0, 0)));
        assert_eq!(matrix.offset(&v2(1, 1), v2(0, -1)), Some(v2(1, 0)));
        assert_eq!(matrix.offset(&v2(1, 1), v2(1, -1)), Some(v2(2, 0)));
        assert_eq!(matrix.offset(&v2(1, 1), v2(1, 0)), Some(v2(2, 1)));
        assert_eq!(matrix.offset(&v2(1, 1), v2(1, 1)), Some(v2(2, 2)));
        assert_eq!(matrix.offset(&v2(1, 1), v2(0, 1)), Some(v2(1, 2)));
        assert_eq!(matrix.offset(&v2(1, 1), v2(-1, 1)), Some(v2(0, 2)));
        assert_eq!(matrix.offset(&v2(1, 1), v2(-1, 0)), Some(v2(0, 1)));
    }

    #[test]
    fn offset_out_of_bounds() {
        let matrix = M::from_element(1, 1, 1.0);
        for delta in [
            v2(-1, -1),
            v2(-1, 0),
            v2(-1, 1),
            v2(0, 1),
            v2(1, 1),
            v2(1, 0),
            v2(1, -1),
            v2(0, -1),
        ]
        .iter()
        {
            assert_eq!(matrix.offset(&v2(0, 0), *delta), None);
        }
    }

    #[test]
    fn neighbours_all_in_bounds() {
        let matrix = M::from_element(3, 3, 1);
        let actual = matrix.neighbours(&v2(1, 1));
        let expected = vec![v2(1, 0), v2(2, 1), v2(1, 2), v2(0, 1)];
        assert!(same_elements(&actual, &expected))
    }

    #[test]
    fn neighbours_some_out_of_bounds() {
        let matrix = M::from_element(3, 3, 1);
        let actual = matrix.neighbours(&v2(0, 0));
        let expected = vec![v2(1, 0), v2(0, 1)];
        assert!(same_elements(&actual, &expected))
    }

    #[test]
    fn test_get_corners() {
        assert_eq!(
            get_corners(&v2(0, 0)),
            [v2(0, 0), v2(1, 0), v2(1, 1), v2(0, 1)]
        );
    }

    #[test]
    fn test_get_corners_in_bound() {
        assert_eq!(get_corners_in_bounds(&v2(1, 1), &2, &2), [v2(1, 1)]);
    }

    #[test]
    fn test_get_corners_in_bounds_grid() {
        let matrix: M<usize> = M::zeros(3, 3);
        assert_eq!(matrix.get_corners_in_bounds(&v2(2, 2)), [v2(2, 2)]);
    }

    #[test]
    fn test_get_adjacent_tiles() {
        let matrix: M<usize> = M::zeros(3, 3);
        assert!(same_elements(
            &matrix.get_adjacent_tiles_in_bounds(&v2(1, 1)),
            &[v2(0, 0), v2(1, 0), v2(1, 1), v2(0, 1)]
        ));
    }

    #[test]
    fn test_get_adjacent_tiles_some_tiles_out_of_bounds() {
        let matrix: M<usize> = M::zeros(3, 3);
        assert!(same_elements(
            &matrix.get_adjacent_tiles_in_bounds(&v2(0, 0)),
            &[v2(0, 0)]
        ));
    }

    #[test]
    fn test_expand() {
        let matrix: M<usize> = M::zeros(3, 3);
        let actual = matrix.expand_position(&v2(1, 1));
        assert_eq!(actual.len(), 9);
        assert!(actual.contains(&v2(0, 0)));
        assert!(actual.contains(&v2(1, 0)));
        assert!(actual.contains(&v2(2, 0)));
        assert!(actual.contains(&v2(0, 1)));
        assert!(actual.contains(&v2(1, 1)));
        assert!(actual.contains(&v2(1, 1)));
        assert!(actual.contains(&v2(0, 2)));
        assert!(actual.contains(&v2(1, 2)));
        assert!(actual.contains(&v2(2, 2)));
    }

    #[test]
    fn test_expand_top_left_corner() {
        let matrix: M<usize> = M::zeros(3, 3);
        let actual = matrix.expand_position(&v2(0, 0));
        assert_eq!(actual.len(), 4);
        assert!(actual.contains(&v2(0, 0)));
        assert!(actual.contains(&v2(1, 0)));
        assert!(actual.contains(&v2(0, 1)));
        assert!(actual.contains(&v2(1, 1)));
    }

    #[test]
    fn test_expand_bottom_right_corner() {
        let matrix: M<usize> = M::zeros(3, 3);
        let actual = matrix.expand_position(&v2(2, 2));
        assert_eq!(actual.len(), 4);
        assert!(actual.contains(&v2(2, 2)));
        assert!(actual.contains(&v2(2, 1)));
        assert!(actual.contains(&v2(1, 2)));
        assert!(actual.contains(&v2(1, 1)));
    }
}
