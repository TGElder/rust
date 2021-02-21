use super::utils::*;
use crate::graphics::Drawing;
use crate::Command;
use color::Color;
use commons::v3;

pub fn draw_sea(id: usize, width: f32, height: f32, level: f32) -> Vec<Command> {
    let color = Color::new(0.0, 0.0, 1.0, 1.0);

    let left = -0.5 * width;
    let right = 1.5 * width;
    let top = -0.5 * height;
    let bottom = 1.5 * height;
    let floats = get_uniform_colored_vertices_from_square(
        &[
            v3(left, top, level),
            v3(right, top, level),
            v3(right, bottom, level),
            v3(left, bottom, level),
        ],
        &color,
    );

    vec![
        Command::CreateDrawing(Drawing::plain(id, floats.len())),
        Command::UpdateVertices {
            id,
            index: 0,
            floats,
        },
    ]
}
