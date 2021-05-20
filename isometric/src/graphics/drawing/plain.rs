use commons::V3;

use crate::coords::WorldCoord;
use crate::drawing::get_uniform_colored_vertices_from_square;
use crate::graphics::Drawing;
use crate::{Color, Command};

pub fn create_plain(name: String, floats: usize) -> Command {
    Command::CreateDrawing(Drawing::plain(name, floats))
}

pub fn draw_rectangle(name: String, coordinates: &[V3<f32>; 4], color: &Color) -> Command {
    let floats = get_uniform_colored_vertices_from_square(coordinates, color);
    Command::UpdateVertices {
        name,
        floats,
        index: 0,
    }
}

pub fn offset_plain_floats(floats: &[f32], target: &mut [f32], offset: &WorldCoord) {
    target.copy_from_slice(floats);
    (0..target.len()).step_by(6).for_each(|i| {
        target[i] += offset.x;
        target[i + 1] += offset.y;
        target[i + 2] += offset.z;
    });
}
