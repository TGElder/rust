use crate::graphics::Drawing;
use crate::Command;
use commons::rectangle::Rectangle;
use coords::WorldCoord;

const BILLBOARD_FLOATS: usize = 42;

pub struct Billboard<'a> {
    pub world_coord: &'a WorldCoord,
    pub width: &'a f32,
    pub height: &'a f32,
    pub texture_coords: &'a Rectangle<f32>,
}

#[rustfmt::skip]
fn get_floats(billboard: Billboard) -> Vec<f32> {
    let w = billboard.world_coord;

    let left = -billboard.width / 2.0;
    let right = -left;
    let top = -billboard.height / 2.0;
    let bottom = -top;

    let t_left = billboard.texture_coords.from.x;
    let t_right = billboard.texture_coords.to.x;
    let t_top = billboard.texture_coords.to.y;
    let t_bottom = billboard.texture_coords.from.y;

    vec![
        w.x, w.y, w.z, t_left, t_top, left, top, 
        w.x, w.y, w.z, t_right, t_bottom, right, bottom,
        w.x, w.y, w.z, t_left, t_bottom, left, bottom, 
        w.x, w.y, w.z, t_left, t_top, left, top,
        w.x, w.y, w.z, t_right, t_top, right, top,
        w.x, w.y, w.z, t_right, t_bottom, right, bottom,
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

pub fn update_billboard_vertices(name: String, billboard: Billboard) -> Vec<Command> {
    let floats = get_floats(billboard);
    vec![Command::UpdateVertices {
        name,
        index: 0,
        floats,
    }]
}

pub fn update_billboards_vertices(name: String, billboards: Vec<Billboard>) -> Command {
    let mut floats = Vec::with_capacity(BILLBOARD_FLOATS * billboards.len());

    for billboard in billboards {
        floats.append(&mut get_floats(billboard));
    }

    Command::UpdateVertices {
        name,
        index: 0,
        floats,
    }
}

pub fn create_and_update_billboards(
    name: String,
    texture: &str,
    billboards: Vec<Billboard>,
) -> Vec<Command> {
    vec![
        create_billboards(name.clone(), billboards.len()),
        update_billboards_vertices(name.clone(), billboards),
        update_billboard_texture(name, texture),
    ]
}
