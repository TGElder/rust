use crate::Command;
use crate::coords::WorldCoord;
use crate::graphics::Drawing;

pub fn create_plain(name: String, floats: usize) -> Command {
    Command::CreateDrawing(Drawing::plain(name, floats))
}

pub fn offset_plain_floats<'a>(floats: &'a [f32], offset: &'a WorldCoord) -> impl Iterator<Item = f32> + 'a {
    (0..floats.len()).step_by(6).flat_map(move |i| 
        vec![
            floats[i] + offset.x,
            floats[i + 1] + offset.y,
            floats[i + 2] + offset.z,
            floats[i + 3],
            floats[i + 4],
            floats[i + 5]
        ].into_iter()
    )
}
