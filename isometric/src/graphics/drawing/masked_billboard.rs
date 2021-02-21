use crate::graphics::Drawing;
use crate::Command;
use color::Color;
use coords::WorldCoord;

const MASKED_BILLBOARD_FLOATS: usize = 66;

#[rustfmt::skip]
fn get_floats(world_coord: WorldCoord, width: f32, height: f32, color: &Color) -> Vec<f32> {
    let p = world_coord;

    let left = -width / 2.0;
    let right = -left;
    let top = -height / 2.0;
    let bottom = -top;

    vec![
        p.x, p.y, p.z, 0.0, 1.0, left, top, color.r, color.g, color.b, color.a,
        p.x, p.y, p.z, 1.0, 0.0, right, bottom, color.r, color.g, color.b, color.a,
        p.x, p.y, p.z, 0.0, 0.0, left, bottom, color.r, color.g, color.b, color.a,
        p.x, p.y, p.z, 0.0, 1.0, left, top, color.r, color.g, color.b, color.a,
        p.x, p.y, p.z, 1.0, 1.0, right, top, color.r, color.g, color.b, color.a,
        p.x, p.y, p.z, 1.0, 0.0, right, bottom, color.r, color.g, color.b, color.a,
    ]
}

pub fn create_masked_billboard(id: usize) -> Command {
    Command::CreateDrawing(Drawing::masked_billboard(id, MASKED_BILLBOARD_FLOATS))
}

pub fn update_masked_billboard_vertices(
    id: usize,
    world_coord: WorldCoord,
    width: f32,
    height: f32,
    color: &Color,
) -> Vec<Command> {
    let floats = get_floats(world_coord, width, height, color);
    vec![Command::UpdateVertices {
        id,
        index: 0,
        floats,
    }]
}

pub fn update_masked_billboard_texture(id: usize, texture: &str) -> Vec<Command> {
    vec![Command::UpdateTexture {
        id,
        texture: Some(texture.to_string()),
    }]
}

pub fn update_masked_billboard_mask(id: usize, texture: &str) -> Vec<Command> {
    vec![Command::UpdateMask {
        id,
        texture: Some(texture.to_string()),
    }]
}
