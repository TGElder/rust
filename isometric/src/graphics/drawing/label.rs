use crate::graphics::{Drawing, LabelVisibilityCheck};
use crate::Command;
use commons::rectangle::Rectangle;
use commons::unsafe_ordering;
use commons::{v2, V2};
use coords::WorldCoord;
use font::Font;

#[rustfmt::skip]
pub fn draw_label(
    id: usize,
    text: &str,
    world_coord: WorldCoord,
    font: &Font,
    draw_order: i32,
) -> Vec<Command> {
    let mut floats = vec![];

    let total_width: f32 = font.get_width(text) as f32;
    let mut x_start = -total_width / 2.0;
    let mut previous = None;
    let mut glyph_positions = vec![];

    for character in text.chars() {
        let texture = font.get_texture_coords(character);
        let dimensions = font.get_dimensions(character).map(|value| value as f32);
        let start = v2(x_start, font.base() - dimensions.y);
        let offset = font.get_offset(character);
        let kerning = previous.map(|previous| font.get_kerning(previous, character)).unwrap_or(0);
        let offset = v2((offset.x + kerning) as f32, -offset.y as f32);
        let position = Rectangle{
            from: start + offset,
            to: start + offset + dimensions
        };

        let p = world_coord;

        floats.append(&mut vec![
            p.x, p.y, p.z, texture.from.x, texture.to.y, position.from.x, position.from.y,
            p.x, p.y, p.z, texture.to.x, texture.from.y, position.to.x, position.to.y,
            p.x, p.y, p.z, texture.from.x, texture.from.y, position.from.x, position.to.y,
            p.x, p.y, p.z, texture.from.x, texture.to.y, position.from.x, position.from.y,
            p.x, p.y, p.z, texture.to.x, texture.to.y, position.to.x, position.from.y,
            p.x, p.y, p.z, texture.to.x, texture.from.y, position.to.x, position.to.y,
        ]);

        x_start += font.get_advance(character) as f32;

        previous = Some(character);

        glyph_positions.push(position.from);
        glyph_positions.push(position.to);
    }

    let visibility_check = get_visibility_check(world_coord, glyph_positions);

    vec![
        Command::CreateDrawing(Drawing::label(
            id,
            floats.len(),
            visibility_check,
            draw_order,
        )),
        Command::UpdateVertices {
            id,
            index: 0,
            floats,
        },
        Command::UpdateTexture {
            id,
            texture: Some(font.texture().to_string()),
        },
    ]
}

fn get_visibility_check(world_coord: WorldCoord, positions: Vec<V2<f32>>) -> LabelVisibilityCheck {
    LabelVisibilityCheck {
        world_coord,
        ui_offsets: Rectangle {
            from: v2(
                positions
                    .iter()
                    .map(|position| position.x)
                    .min_by(unsafe_ordering)
                    .unwrap_or_default(),
                positions
                    .iter()
                    .map(|position| position.y)
                    .min_by(unsafe_ordering)
                    .unwrap_or_default(),
            ),
            to: v2(
                positions
                    .iter()
                    .map(|position| position.x)
                    .max_by(unsafe_ordering)
                    .unwrap_or_default(),
                positions
                    .iter()
                    .map(|position| position.y)
                    .max_by(unsafe_ordering)
                    .unwrap_or_default(),
            ),
        },
    }
}
