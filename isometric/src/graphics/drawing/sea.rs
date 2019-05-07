use super::super::engine::DrawingType;
use super::super::vertex_objects::VBO;
use super::utils::*;
use super::Drawing;
use color::Color;
use coords::WorldCoord;
use v3;

pub struct SeaDrawing {
    vbo: VBO,
}

impl Drawing for SeaDrawing {
    fn draw(&self) {
        self.vbo.draw();
    }

    fn get_z_mod(&self) -> f32 {
        0.0
    }

    fn drawing_type(&self) -> &DrawingType {
        self.vbo.drawing_type()
    }

    fn get_visibility_check_coord(&self) -> Option<&WorldCoord> {
        None
    }
}

impl SeaDrawing {
    pub fn new(width: f32, height: f32, level: f32) -> SeaDrawing {
        let mut vbo = VBO::new(DrawingType::Plain);

        let color = Color::new(0.0, 0.0, 1.0, 1.0);

        let left = -0.5 * width;
        let right = 1.5 * width;
        let top = -0.5 * height;
        let bottom = 1.5 * height;
        vbo.load(get_uniform_colored_vertices_from_square(
            &[
                v3(left, top, level),
                v3(right, top, level),
                v3(right, bottom, level),
                v3(left, bottom, level),
            ],
            &color,
        ));

        SeaDrawing { vbo }
    }
}
