pub struct Scale {
    in_range: (f64, f64),
    out_range: (f64, f64),
}

impl Scale {
    pub fn new(in_range: (f64, f64), out_range: (f64, f64)) -> Scale {
        Scale {
            in_range,
            out_range,
        }
    }

    pub fn scale(&self, value: f64) -> f64 {
        return ((value - self.in_range.0) / (self.in_range.1 - self.in_range.0))
            * (self.out_range.1 - self.out_range.0)
            + self.out_range.0;
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_scale() {
        let scale = Scale::new((2000.0, 2016.0), (11.0, 55.0));
        assert_eq!(scale.scale(2003.0), 19.25);
    }

}
