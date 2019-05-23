use crate::graphics::Drawing;
use crate::Command;
use coords::WorldCoord;

#[rustfmt::skip]
pub fn draw_billboard(name: String, world_coord: WorldCoord, width: f32, height: f32, texture: &str) -> Vec<Command> {
    let p = world_coord;

    let left = -width / 2.0;
    let right = -left;
    let top = -height / 2.0;
    let bottom = -top;

    let floats = vec![
        p.x, p.y, p.z, 0.0, 1.0, left, top,
        p.x, p.y, p.z, 0.0, 0.0, left, bottom,
        p.x, p.y, p.z, 1.0, 0.0, right, bottom,
        p.x, p.y, p.z, 0.0, 1.0, left, top,
        p.x, p.y, p.z, 1.0, 0.0, right, bottom,
        p.x, p.y, p.z, 1.0, 1.0, right, top,
    ];

    vec![
        Command::CreateDrawing(Drawing::billboard(name.clone(), floats.len(), texture.to_string())),
        Command::UpdateDrawing{
            name,
            index: 0,
            floats,
        }
    ]
}
