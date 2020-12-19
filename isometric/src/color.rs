use commons::rand::prelude::*;
use serde::{Deserialize, Serialize};
use std::ops::Mul;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Mul for Color {
    type Output = Self;

    fn mul(self, other: Self) -> Self {
        Color::new(
            self.r * other.r,
            self.g * other.g,
            self.b * other.b,
            self.a * other.a,
        )
    }
}

impl Color {
    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Color {
        Color { r, g, b, a }
    }

    pub fn random<T: Rng>(rng: &mut T, a: f32) -> Color {
        Color::new(
            rng.gen_range(0.0..1.0),
            rng.gen_range(0.0..1.0),
            rng.gen_range(0.0..1.0),
            a,
        )
    }

    pub fn mul(&self, other: &Color) -> Color {
        Color::new(
            self.r * other.r,
            self.g * other.g,
            self.b * other.b,
            self.a * other.a,
        )
    }

    pub fn blend(&self, p: f32, other: &Color) -> Color {
        let pc = 1.0 - p;
        Color::new(
            self.r * p + other.r * pc,
            self.g * p + other.g * pc,
            self.b * p + other.b * pc,
            self.a * p + other.a * pc,
        )
    }

    pub fn layer_over(&self, other: &Color) -> Color {
        let a = self.a;
        let ac = 1.0 - a;
        Color::new(
            self.r * a + other.r * ac,
            self.g * a + other.g * ac,
            self.b * a + other.b * ac,
            1.0,
        )
    }

    pub fn transparent() -> Color {
        Color::new(0.0, 0.0, 0.0, 0.0)
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use commons::almost::Almost;

    #[test]
    fn test_mul() {
        let result = Color::new(0.1, 0.2, 0.3, 0.4) * Color::new(0.5, 0.6, 0.7, 0.8);
        assert!(result.r.almost(&(0.1 * 0.5)));
        assert!(result.g.almost(&(0.2 * 0.6)));
        assert!(result.b.almost(&(0.3 * 0.7)));
        assert!(result.a.almost(&(0.4 * 0.8)));
    }

    #[test]
    fn test_blend() {
        let result = Color::new(0.1, 0.2, 0.3, 0.4).blend(0.2, &Color::new(0.5, 0.6, 0.7, 0.8));
        assert!(result.r.almost(&(0.1 * 0.2 + 0.5 * 0.8)));
        assert!(result.g.almost(&(0.2 * 0.2 + 0.6 * 0.8)));
        assert!(result.b.almost(&(0.3 * 0.2 + 0.7 * 0.8)));
        assert!(result.a.almost(&(0.4 * 0.2 + 0.8 * 0.8)));
    }

    #[test]
    fn test_layer_over() {
        let result = Color::new(0.1, 0.2, 0.3, 0.4).layer_over(&Color::new(0.5, 0.6, 0.7, 0.8));
        assert!(result.r.almost(&(0.1 * 0.4 + 0.5 * 0.6)));
        assert!(result.g.almost(&(0.2 * 0.4 + 0.6 * 0.6)));
        assert!(result.b.almost(&(0.3 * 0.4 + 0.7 * 0.6)));
        assert!(result.a.almost(&1.0));
    }
}
