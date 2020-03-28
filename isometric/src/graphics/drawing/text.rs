use crate::graphics::Drawing;
use crate::Command;
use coords::WorldCoord;
use font::Font;

#[rustfmt::skip]
pub fn draw_text(name: String, text: &str, world_coord: WorldCoord, font: &Font) -> Vec<Command> {
    let mut floats = vec![];

    let total_width: f32 = font.get_width(text) as f32;
    let mut xs = -total_width / 2.0;

    for character in text.chars() {
        let (top_left, bottom_right) = font.get_texture_coords(character);
        let p = world_coord;
        let (w, h) = font.get_dimensions(character);
        let (w, h) = (w as f32, h as f32);

        floats.append(&mut vec![
            p.x, p.y, p.z, top_left.x, bottom_right.y, xs, 0.0,
            p.x, p.y, p.z, top_left.x, top_left.y, xs, h,
            p.x, p.y, p.z, bottom_right.x, top_left.y, xs + w, h,
            p.x, p.y, p.z, top_left.x, bottom_right.y, xs, 0.0,
            p.x, p.y, p.z, bottom_right.x, top_left.y, xs + w, h,
            p.x, p.y, p.z, bottom_right.x, bottom_right.y, xs + w, 0.0,
        ]);

        xs += font.get_advance(character) as f32;
    }

    vec![
        Command::CreateDrawing(Drawing::text(name.clone(), floats.len(), font, world_coord)),
        Command::UpdateDrawing{
            name: name,
            index: 0,
            floats,
        }
    ]
}
