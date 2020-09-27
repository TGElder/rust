use crate::grid::Grid;
use crate::scale::Scale;
use crate::{v2, V2};
use num::{cast, Float};
use std::cmp::Ordering;
use std::fmt::Debug;

#[derive(Debug)]
pub struct PositionValue<T>
where
    T: 'static + Float + Debug,
{
    pub position: V2<usize>,
    pub value: T,
}

impl<T> Ord for PositionValue<T>
where
    T: 'static + Float + Debug,
{
    fn cmp(&self, other: &PositionValue<T>) -> Ordering {
        self.value.partial_cmp(&other.value).unwrap()
    }
}

impl<T> PartialOrd for PositionValue<T>
where
    T: 'static + Float + Debug,
{
    fn partial_cmp(&self, other: &PositionValue<T>) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> Eq for PositionValue<T> where T: 'static + Float + Debug {}

impl<T> PartialEq for PositionValue<T>
where
    T: 'static + Float + Debug,
{
    fn eq(&self, other: &PositionValue<T>) -> bool {
        self.value == other.value
    }
}

pub fn equalize<T, U: Grid<T>>(grid: U) -> U
where
    T: 'static + Float + Debug,
{
    equalize_with_filter(grid, &|_| true)
}

pub fn equalize_with_filter<T, U: Grid<T>>(
    mut grid: U,
    filter: &dyn Fn(&PositionValue<T>) -> bool,
) -> U
where
    T: 'static + Float + Debug,
{
    let width = grid.width();
    let height = grid.height();

    let mut sorted = Vec::with_capacity(width * height);

    for x in 0..width {
        for y in 0..height {
            let position = v2(x, y);
            let value = grid.get_cell_unsafe(&position);
            let position_value = PositionValue {
                position,
                value: *value,
            };
            if filter(&position_value) {
                sorted.push(position_value);
            }
        }
    }
    sorted.sort();

    let scale: Scale<T> = Scale::new(
        (T::zero(), cast(sorted.len() - 1).unwrap()),
        (T::zero(), T::one()),
    );

    for (i, pv) in sorted.into_iter().enumerate() {
        let equalized = scale.scale(cast(i).unwrap());
        *grid.mut_cell_unsafe(&pv.position) = equalized;
    }
    grid
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::almost::Almost;
    use crate::{same_elements, M};

    #[test]
    fn test_equalize() {
        let input = M::from_vec(3, 3, vec![5.0, 10.0, 50.0, 30.0, 2.0, 4.0, 40.0, 3.0, 1.0]);

        let actual = equalize(input);

        let expected = M::from_vec(
            3,
            3,
            vec![0.5, 0.625, 1.0, 0.75, 0.125, 0.375, 0.875, 0.25, 0.0],
        );

        assert!(actual.almost(&expected));
    }

    #[test]
    fn test_equalize_duplicates() {
        let input = M::from_vec(3, 3, vec![0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]);

        let actual: Vec<f64> = equalize(input).iter().cloned().collect();

        let expected: Vec<f64> = M::from_vec(
            3,
            3,
            vec![0.5, 0.625, 1.0, 0.75, 0.125, 0.375, 0.875, 0.25, 0.0],
        )
        .iter()
        .cloned()
        .collect();

        assert!(same_elements(&actual, &expected));
    }

    #[test]
    fn test_equalize_with_filter() {
        let input = M::from_vec(
            3,
            3,
            vec![10.0, 2.0, 0.0, 102.0, 101.0, 0.0, 0.0, 3.0, 100.0],
        );

        let actual: Vec<f64> =
            equalize_with_filter(input, &|PositionValue { value, .. }| *value != 0.0)
                .iter()
                .cloned()
                .collect();

        let expected: Vec<f64> =
            M::from_vec(3, 3, vec![0.4, 0.0, 0.0, 1.0, 0.8, 0.0, 0.0, 0.2, 0.6])
                .iter()
                .cloned()
                .collect();

        assert!(same_elements(&actual, &expected));
    }
}
