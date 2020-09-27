use crate::{M, V2, V3};
use float_cmp::approx_eq;
use std::fmt::Debug;

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

impl<T> Almost for M<T>
where
    T: Almost + Copy + Debug + PartialEq + 'static,
{
    fn almost(&self, other: &M<T>) -> bool {
        self.iter()
            .enumerate()
            .all(|(i, value)| value.almost(&other[i]))
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    use crate::{v2, v3};

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

    #[test]
    fn test_almost_m() {
        let a = M::from_vec(1, 3, vec![0.1, 0.2, 0.3]);
        let b = M::from_vec(1, 3, vec![0.1, 0.2, 0.3]);
        assert!(a.almost(&b));
    }
}
