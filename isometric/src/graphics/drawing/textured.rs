use crate::color::*;
use crate::graphics::Drawing;
use crate::Command;
use coords::WorldCoord;

#[rustfmt::skip]
pub fn draw_textured(name: String, color: &Color, texture: &str, corners: [WorldCoord; 4]) -> Vec<Command> {
    let [a, b, c, d] = corners;
    let floats = vec![
        a.x, a.y, a.z, color.r, color.g, color.b, color.a, 0.0, 0.0,
        d.x, d.y, d.z, color.r, color.g, color.b, color.a, 0.0, 1.0,
        c.x, c.y, c.z, color.r, color.g, color.b, color.a, 1.0, 1.0,
        a.x, a.y, a.z, color.r, color.g, color.b, color.a, 0.0, 0.0,
        c.x, c.y, c.z, color.r, color.g, color.b, color.a, 1.0, 1.0,
        b.x, b.y, b.z, color.r, color.g, color.b, color.a, 1.0, 0.0,
    ];
    vec![
        Command::CreateDrawing(Drawing::textured(
            name.clone(),
            floats.len(),
        )),
        Command::UpdateVertices {
            name: name.clone(),
            index: 0,
            floats,
        },
        Command::UpdateTexture {
            name,
            texture: Some(texture.to_string()),
        }
    ]
}
