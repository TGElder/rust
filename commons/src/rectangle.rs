use crate::V2;
use std::fmt::Debug;

#[derive(Debug)]
pub struct Rectangle<T>
where
    T: Copy + Debug + PartialOrd + 'static,
{
    pub from: V2<T>,
    pub to: V2<T>,
}

impl<T> Rectangle<T>
where
    T: Copy + Debug + PartialOrd + 'static,
{
    pub fn new(from: V2<T>, to: V2<T>) -> Rectangle<T> {
        Rectangle { from, to }
    }

    pub fn overlaps(&self, other: &Rectangle<T>) -> bool {
        self.from.x < other.to.x
            && other.from.x < self.to.x
            && self.from.y < other.to.y
            && other.from.y < self.to.y
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::v2;

    #[test]
    fn rectangles_overlap_a_inside_b() {
        let a = Rectangle::new(v2(1, 1), v2(2, 2));
        let b = Rectangle::new(v2(0, 0), v2(3, 3));

        assert!(a.overlaps(&b));
        assert!(b.overlaps(&a));
    }

    #[test]
    fn rectangles_overlap_b_inside_a() {
        let a = Rectangle::new(v2(0, 0), v2(3, 3));
        let b = Rectangle::new(v2(1, 1), v2(2, 2));

        assert!(a.overlaps(&b));
        assert!(b.overlaps(&a));
    }

    #[test]
    fn rectangles_overlap_bottom_left_of_a() {
        let a = Rectangle::new(v2(1, 1), v2(3, 3));
        let b = Rectangle::new(v2(0, 0), v2(2, 2));

        assert!(a.overlaps(&b));
        assert!(b.overlaps(&a));
    }

    #[test]
    fn rectangles_overlap_top_left_of_a() {
        let a = Rectangle::new(v2(1, 1), v2(3, 3));
        let b = Rectangle::new(v2(0, 2), v2(2, 4));

        assert!(a.overlaps(&b));
        assert!(b.overlaps(&a));
    }

    #[test]
    fn rectangles_overlap_bottom_right_of_a() {
        let a = Rectangle::new(v2(1, 1), v2(3, 3));
        let b = Rectangle::new(v2(2, 0), v2(4, 2));

        assert!(a.overlaps(&b));
        assert!(b.overlaps(&a));
    }

    #[test]
    fn rectangles_overlap_top_right_of_a() {
        let a = Rectangle::new(v2(1, 1), v2(3, 3));
        let b = Rectangle::new(v2(2, 2), v2(4, 4));

        assert!(a.overlaps(&b));
        assert!(b.overlaps(&a));
    }

    #[test]
    fn rectangles_do_not_overlap_a_below_b() {
        let a = Rectangle::new(v2(0, 0), v2(1, 1));
        let b = Rectangle::new(v2(0, 2), v2(1, 3));

        assert!(!a.overlaps(&b));
        assert!(!b.overlaps(&a));
    }

    #[test]
    fn rectangles_do_not_overlap_a_left_of_b() {
        let a = Rectangle::new(v2(0, 0), v2(1, 1));
        let b = Rectangle::new(v2(2, 0), v2(3, 1));

        assert!(!a.overlaps(&b));
        assert!(!b.overlaps(&a));
    }

    #[test]
    fn rectangles_overlaps_itself() {
        let a = Rectangle::new(v2(0, 0), v2(1, 1));

        assert!(a.overlaps(&a));
    }
}
