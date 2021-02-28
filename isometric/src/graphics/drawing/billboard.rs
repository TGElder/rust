use crate::graphics::Drawing;
use crate::Command;
use coords::WorldCoord;

const BILLBOARD_FLOATS: usize = 42;

#[rustfmt::skip]
fn get_floats(world_coord: WorldCoord, width: f32, height: f32) -> Vec<f32> {
    let p = world_coord;

    let left = -width / 2.0;
    let right = -left;
    let top = -height / 2.0;
    let bottom = -top;

    vec![
        p.x, p.y, p.z, 0.0, 1.0, left, top, 
        p.x, p.y, p.z, 1.0, 0.0, right, bottom,
        p.x, p.y, p.z, 0.0, 0.0, left, bottom, 
        p.x, p.y, p.z, 0.0, 1.0, left, top,
        p.x, p.y, p.z, 1.0, 1.0, right, top,
        p.x, p.y, p.z, 1.0, 0.0, right, bottom,
    ]
}

pub fn create_billboard(name: String) -> Command {
    Command::CreateDrawing(Drawing::billboard(name, BILLBOARD_FLOATS))
}


pub fn create_billboards(name: String, count: usize) -> Command {
    Command::CreateDrawing(Drawing::billboard(name, BILLBOARD_FLOATS * count))
}

pub fn update_billboard_texture(name: String, texture: &str) -> Command {
    Command::UpdateTexture {
        name,
        texture: Some(texture.to_string()),
    }
}


pub fn update_billboard_vertices(
    name: String,
    world_coord: WorldCoord,
    width: f32,
    height: f32,
) -> Vec<Command> {
    let floats = get_floats(world_coord, width, height);
    vec![Command::UpdateVertices {
        name,
        index: 0,
        floats,
    }]
}


pub fn update_billboards_vertices(name: String, world_coords: Vec<WorldCoord>, width: f32, height: f32) -> Command {
    let mut floats = vec![];

    for world_coord in world_coords {
        floats.append(&mut get_floats(world_coord, width, height));
    }

    Command::UpdateVertices {
        name,
        index: 0,
        floats,
    }
}

pub fn create_and_update_billboards(
    name: String,
    world_coords: Vec<WorldCoord>,
    width: f32,
    height: f32,
    texture: &str,
) -> Vec<Command> {
    vec![
        create_billboards(name.clone(), world_coords.len()),
        update_billboards_vertices(name.clone(), world_coords, width, height),
        update_billboard_texture(name, texture),
    ]
}
