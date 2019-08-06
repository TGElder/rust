use crate::graphics::Drawing;
use crate::Command;
use coords::WorldCoord;

fn get_floats(world_coord: WorldCoord, width: f32, height: f32) -> Vec<f32> {
    let p = world_coord;

    let left = -width / 2.0;
    let right = -left;
    let top = -height / 2.0;
    let bottom = -top;

    vec![
        p.x, p.y, p.z, 0.0, 1.0, left, top, p.x, p.y, p.z, 0.0, 0.0, left, bottom, p.x, p.y, p.z,
        1.0, 0.0, right, bottom, p.x, p.y, p.z, 0.0, 1.0, left, top, p.x, p.y, p.z, 1.0, 0.0,
        right, bottom, p.x, p.y, p.z, 1.0, 1.0, right, top,
    ]
}

#[rustfmt::skip]
pub fn draw_billboard(name: String, world_coord: WorldCoord, width: f32, height: f32, texture: &str) -> Vec<Command> {
    let floats = get_floats(world_coord, width, height);
    vec![
        Command::CreateDrawing(Drawing::billboard(name.clone(), floats.len(), texture.to_string())),
        Command::UpdateDrawing{
            name,
            index: 0,
            floats,
        }
    ]
}

#[rustfmt::skip]
pub fn draw_billboards(name: String, world_coords: Vec<WorldCoord>, width: f32, height: f32, texture: &str) -> Vec<Command> {

    let mut floats = vec![];

    for world_coord in world_coords {
        floats.append(&mut get_floats(world_coord, width, height));
    }

    vec![
        Command::CreateDrawing(Drawing::billboard(name.clone(), floats.len(), texture.to_string())),
        Command::UpdateDrawing{
            name,
            index: 0,
            floats,
        }
    ]
}
