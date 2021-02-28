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

pub fn create_masked_billboard(name: String) -> Command {
    Command::CreateDrawing(Drawing::billboard(name, MASKED_BILLBOARD_FLOATS))
}


pub fn create_masked_billboards(name: String, count: usize) -> Command {
    Command::CreateDrawing(Drawing::billboard(name, MASKED_BILLBOARD_FLOATS * count))
}

pub fn update_masked_billboard_texture(name: String, texture: &str) -> Command {
    Command::UpdateTexture {
        name,
        texture: Some(texture.to_string()),
    }
}

pub fn update_masked_billboard_mask(name: String, texture: &str) -> Command {
    Command::UpdateMask {
        name,
        texture: Some(texture.to_string()),
    }
}

pub fn update_masked_billboard_vertices(
    name: String,
    world_coord: WorldCoord,
    width: f32,
    height: f32,
    color: &Color,
) -> Vec<Command> {
    let floats = get_floats(world_coord, width, height, &color);
    vec![Command::UpdateVertices {
        name,
        index: 0,
        floats,
    }]
}


pub fn update_masked_billboards_vertices(name: String, world_coords: Vec<WorldCoord>, colors: Vec<&Color>, width: f32, height: f32) -> Command {
    let mut floats = vec![];

    for (world_coord, color) in world_coords.into_iter().zip(colors.iter()) {
        floats.append(&mut get_floats(world_coord, width, height, color));
    }

    Command::UpdateVertices {
        name,
        index: 0,
        floats,
    }
}