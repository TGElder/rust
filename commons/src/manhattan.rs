use std::fmt::Debug;
use std::ops::{Add, Sub};
use V2;

pub trait ManhattanDistance<T>
where
    T: Add<Output = T> + Sub<Output = T> + Copy + Debug + PartialOrd + Sub + Add + 'static,
{
    fn manhattan_distance(&self, other: &V2<T>) -> T;
}

impl<T> ManhattanDistance<T> for V2<T>
where
    T: Add<Output = T> + Sub<Output = T> + Copy + Debug + PartialOrd + Sub + Add + 'static,
{
    fn manhattan_distance(&self, other: &V2<T>) -> T {
        let x_distance = if self.x >= other.x {
            self.x - other.x
        } else {
            other.x - self.x
        };
        let y_distance = if self.y >= other.y {
            self.y - other.y
        } else {
            other.y - self.y
        };
        x_distance + y_distance
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use v2;

    #[test]
    fn two_identical_points() {
        let a = v2(0, 0);
        let b = v2(0, 0);
        let expected = 0;

        assert_eq!(a.manhattan_distance(&b), expected);
        assert_eq!(b.manhattan_distance(&a), expected);
    }

    #[test]
    fn two_a_x_greater() {
        let a = v2(1, 0);
        let b = v2(0, 0);
        let expected = 1;

        assert_eq!(a.manhattan_distance(&b), expected);
        assert_eq!(b.manhattan_distance(&a), expected);
    }

    #[test]
    fn two_b_x_greater() {
        let a = v2(0, 0);
        let b = v2(1, 0);
        let expected = 1;

        assert_eq!(a.manhattan_distance(&b), expected);
        assert_eq!(b.manhattan_distance(&a), expected);
    }

    #[test]
    fn two_a_y_greater() {
        let a = v2(0, 1);
        let b = v2(0, 0);
        let expected = 1;

        assert_eq!(a.manhattan_distance(&b), expected);
        assert_eq!(b.manhattan_distance(&a), expected);
    }

    #[test]
    fn two_b_y_greater() {
        let a = v2(0, 0);
        let b = v2(0, 1);
        let expected = 1;

        assert_eq!(a.manhattan_distance(&b), expected);
        assert_eq!(b.manhattan_distance(&a), expected);
    }

    #[test]
    fn all_different() {
        let a = v2(0, 3);
        let b = v2(4, 2);
        let expected = 5;

        assert_eq!(a.manhattan_distance(&b), expected);
        assert_eq!(b.manhattan_distance(&a), expected);
    }
}
