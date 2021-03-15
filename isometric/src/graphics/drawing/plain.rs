use crate::coords::WorldCoord;
use crate::graphics::Drawing;
use crate::Command;

pub fn create_plain(name: String, floats: usize) -> Command {
    Command::CreateDrawing(Drawing::plain(name, floats))
}

pub fn offset_plain_floats(floats: &[f32], target: &mut [f32], offset: &WorldCoord) {
    target.copy_from_slice(floats);
    (0..target.len()).step_by(6).for_each(|i| {
        target[i] += offset.x;
        target[i + 1] += offset.y;
        target[i + 2] += offset.z;
    });
}
