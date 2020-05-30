use crate::graphics::{Drawing, LabelVisibilityCheck};
use crate::Command;
use commons::rectangle::Rectangle;
use commons::v2;
use coords::WorldCoord;
use font::Font;

#[rustfmt::skip]
pub fn draw_label(name: String, text: &str, world_coord: WorldCoord, font: &Font) -> Vec<Command> {
    let mut floats = vec![];

    let total_width: f32 = font.get_width(text) as f32;
    let mut xs = -total_width / 2.0;
    let mut height = 0.0f32;

    for character in text.chars() {
        let (top_left, bottom_right) = font.get_texture_coords(character);
        let p = world_coord;
        let (w, h) = font.get_dimensions(character);
        let (w, h) = (w as f32, h as f32);
        height = height.max(h);

        floats.append(&mut vec![
            p.x, p.y, p.z, top_left.x, bottom_right.y, xs, 0.0,
            p.x, p.y, p.z, bottom_right.x, top_left.y, xs + w, h,
            p.x, p.y, p.z, top_left.x, top_left.y, xs, h,
            p.x, p.y, p.z, top_left.x, bottom_right.y, xs, 0.0,
            p.x, p.y, p.z, bottom_right.x, bottom_right.y, xs + w, 0.0,
            p.x, p.y, p.z, bottom_right.x, top_left.y, xs + w, h,
        ]);

        xs += font.get_advance(character) as f32;
    }

    let visibility_check = LabelVisibilityCheck{
        world_coord,
        ui_offsets: Rectangle{
            from: v2(-total_width / 2.0, 0.0),
            to: v2(total_width / 2.0, height),
        }
    };

    vec![
        Command::CreateDrawing(Drawing::label(name.clone(), floats.len(), visibility_check)),
        Command::UpdateVertices{
            name: name.clone(),
            index: 0,
            floats,
        },
        Command::UpdateTexture{name, texture: Some(font.texture().clone())},
    ]
}
