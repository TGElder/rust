use color::Color;
use commons::na;
use commons::unsafe_ordering;
use std::f32;

pub trait TriangleColoring: Send {
    fn get_colors(&self, points: &[na::Vector3<f32>; 3]) -> [Color; 3];
}

pub trait SquareColoring: Send {
    fn get_colors(&self, points: &[na::Vector3<f32>; 4]) -> [Color; 4];
}

pub struct AltitudeSquareColoring {
    max_height: f32,
}

impl AltitudeSquareColoring {
    pub fn new(heights: &na::DMatrix<f32>) -> AltitudeSquareColoring {
        let max_height = heights.iter().max_by(unsafe_ordering).unwrap();
        AltitudeSquareColoring {
            max_height: *max_height,
        }
    }
}

impl SquareColoring for AltitudeSquareColoring {
    fn get_colors(&self, points: &[na::Vector3<f32>; 4]) -> [Color; 4] {
        let get_color = |point: na::Vector3<f32>| {
            let color = (point.z / (self.max_height * 2.0)) + 0.5;
            Color::new(color, color, color, 1.0)
        };
        [
            get_color(points[0]),
            get_color(points[1]),
            get_color(points[2]),
            get_color(points[3]),
        ]
    }
}

pub struct AngleTriangleColoring {
    base_color: Color,
    light_direction: na::Vector3<f32>,
}

impl AngleTriangleColoring {
    pub fn new(base_color: Color, light_direction: na::Vector3<f32>) -> AngleTriangleColoring {
        AngleTriangleColoring {
            base_color,
            light_direction,
        }
    }
}

impl TriangleColoring for AngleTriangleColoring {
    fn get_colors(&self, points: &[na::Vector3<f32>; 3]) -> [Color; 3] {
        let u = points[0] - points[1];
        let v = points[0] - points[2];
        let normal = u.cross(&v);
        let angle: f32 = na::Matrix::angle(&normal, &self.light_direction);
        let color = angle / (f32::consts::PI / 2.0);
        let color = Color::new(
            self.base_color.r * color,
            self.base_color.g * color,
            self.base_color.b * color,
            1.0,
        );
        [color; 3]
    }
}

pub struct AngleSquareColoring {
    base_color: Color,
    light_direction: na::Vector3<f32>,
}

impl AngleSquareColoring {
    pub fn new(base_color: Color, light_direction: na::Vector3<f32>) -> AngleSquareColoring {
        AngleSquareColoring {
            base_color,
            light_direction,
        }
    }
}

impl SquareColoring for AngleSquareColoring {
    fn get_colors(&self, points: &[na::Vector3<f32>; 4]) -> [Color; 4] {
        let u = points[0] - points[2];
        let v = points[1] - points[3];
        let normal = u.cross(&v);
        let angle: f32 = na::Matrix::angle(&normal, &self.light_direction);
        let color = angle / (f32::consts::PI / 2.0);
        let color = Color::new(
            self.base_color.r * color,
            self.base_color.g * color,
            self.base_color.b * color,
            1.0,
        );
        [color; 4]
    }
}

#[rustfmt::skip]
pub fn get_uniform_colored_vertices_from_triangle(points: &[na::Vector3<f32>; 3], color: &Color) -> Vec<f32> {
    vec![
        points[0].x, points[0].y, points[0].z, color.r, color.g, color.b,
        points[1].x, points[1].y, points[1].z, color.r, color.g, color.b,
        points[2].x, points[2].y, points[2].z, color.r, color.g, color.b,
    ]
}

#[rustfmt::skip]
pub fn get_specific_colored_vertices_from_triangle(points: &[na::Vector3<f32>; 3], colors: &[Color; 3]) -> Vec<f32> {
    vec![
        points[0].x, points[0].y, points[0].z, colors[0].r, colors[0].g, colors[0].b,
        points[1].x, points[1].y, points[1].z, colors[1].r, colors[1].g, colors[1].b,
        points[2].x, points[2].y, points[2].z, colors[2].r, colors[2].g, colors[2].b,
    ]
}

#[rustfmt::skip]
pub fn get_colored_vertices_from_triangle(points: &[na::Vector3<f32>; 3], coloring: &Box<TriangleColoring>) -> Vec<f32> {
    let colors = coloring.get_colors(&points);

    vec![
        points[0].x, points[0].y, points[0].z, colors[0].r, colors[0].g, colors[0].b,
        points[1].x, points[1].y, points[1].z, colors[1].r, colors[1].g, colors[1].b,
        points[2].x, points[2].y, points[2].z, colors[2].r, colors[2].g, colors[2].b,
    ]
}

#[rustfmt::skip]
pub fn get_uniform_colored_vertices_from_square(points: &[na::Vector3<f32>; 4], color: &Color) -> Vec<f32> {
    [
        [points[0], points[3], points[2]],
        [points[0], points[2], points[1]]
    ].iter().flat_map(|points| get_uniform_colored_vertices_from_triangle(points, color)).collect()
}

#[rustfmt::skip]
pub fn get_specific_colored_vertices_from_square(points: &[na::Vector3<f32>; 4], colors: &[Color; 4]) -> Vec<f32> {
    [
        ([points[0], points[3], points[2]], [colors[0], colors[3], colors[2]]),
        ([points[0], points[2], points[1]], [colors[0], colors[2], colors[1]])
    ].iter().flat_map(|(points, colors)| get_specific_colored_vertices_from_triangle(points, colors)).collect()
}

#[rustfmt::skip]
pub fn get_colored_vertices_from_square(points: &[na::Vector3<f32>; 4], coloring: &Box<SquareColoring>) -> Vec<f32> {
    let colors = coloring.get_colors(&points);

    vec![
        points[0].x, points[0].y, points[0].z, colors[0].r, colors[0].g, colors[0].b,
        points[3].x, points[3].y, points[3].z, colors[3].r, colors[3].g, colors[3].b,
        points[2].x, points[2].y, points[2].z, colors[2].r, colors[2].g, colors[2].b,
        points[0].x, points[0].y, points[0].z, colors[0].r, colors[0].g, colors[0].b,
        points[2].x, points[2].y, points[2].z, colors[2].r, colors[2].g, colors[2].b,
        points[1].x, points[1].y, points[1].z, colors[1].r, colors[1].g, colors[1].b,
    ]
}
