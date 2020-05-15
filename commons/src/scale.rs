use num::Float;

#[derive(Debug, PartialEq)]
pub struct Scale<T: Float> {
    in_range: (T, T),
    out_range: (T, T),
}

impl<T: Float> Scale<T> {
    pub fn new(in_range: (T, T), out_range: (T, T)) -> Scale<T> {
        Scale {
            in_range,
            out_range,
        }
    }

    pub fn scale(&self, value: T) -> T {
        ((value - self.in_range.0) / (self.in_range.1 - self.in_range.0))
            * (self.out_range.1 - self.out_range.0)
            + self.out_range.0
    }

    pub fn inside_range(&self, value: T) -> bool {
        value >= self.in_range.0 && value <= self.in_range.1
    }

    pub fn in_range(&self) -> (T, T) {
        self.in_range
    }

    pub fn out_range(&self) -> (T, T) {
        self.out_range
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use almost::Almost;

    #[test]
    fn test_scale() {
        let scale = Scale::<f32>::new((2000.0, 2016.0), (11.0, 55.0));
        assert!(scale.scale(2003.0).almost(&19.25));
    }

    #[test]
    fn test_inside_range() {
        let scale = Scale::new((2000.0, 2016.0), (11.0, 55.0));
        assert!(scale.inside_range(2003.0));
    }

    #[test]
    fn test_below_range() {
        let scale = Scale::new((2000.0, 2016.0), (11.0, 55.0));
        assert!(!scale.inside_range(1999.0));
    }

    #[test]
    fn test_above_range() {
        let scale = Scale::new((2000.0, 2016.0), (11.0, 55.0));
        assert!(!scale.inside_range(2019.0));
    }
}
