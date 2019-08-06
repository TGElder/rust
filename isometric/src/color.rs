#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Color {
        Color { r, g, b, a }
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
        Color::new(
            self.r * p + other.r * (1.0 - p),
            self.g * p + other.g * (1.0 - p),
            self.b * p + other.b * (1.0 - p),
            self.a * p + other.a * (1.0 - p),
        )
    }
}
