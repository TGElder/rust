use super::utils::*;
use crate::graphics::Drawing;
use crate::Command;
use color::Color;
use commons::{na, v3, V3};
use coords::*;

const BOAT_FLOATS: usize = 540;

pub struct DrawBoatParams {
    pub width: f32,
    pub side_height: f32,
    pub bow_length: f32,
    pub mast_height: f32,
    pub base_color: Color,
    pub sail_color: Color,
    pub light_direction: V3<f32>,
}

pub fn create_boat(name: String) -> Command {
    Command::CreateDrawing(Drawing::plain(name, BOAT_FLOATS))
}

pub fn draw_boat(
    name: &str,
    world_coordinate: WorldCoord,
    rotation: na::Matrix3<f32>,
    p: &DrawBoatParams,
) -> Vec<Command> {
    let triangle_coloring = AngleTriangleColoring::new(p.base_color, p.light_direction);
    let square_coloring = AngleSquareColoring::new(p.base_color, p.light_direction);

    let WorldCoord { x, y, z } = world_coordinate;

    let world_coordinate = v3(x, y, z + 0.01);

    let width_2 = p.width / 2.0;

    let al = (rotation * v3(-width_2, -width_2, 0.0)) + world_coordinate;
    let bl = (rotation * v3(2.0 * width_2, -width_2, 0.0)) + world_coordinate;
    let cl = (rotation * v3(2.0 * width_2, width_2, 0.0)) + world_coordinate;
    let dl = (rotation * v3(-width_2, width_2, 0.0)) + world_coordinate;
    let ah = (rotation * v3(-width_2, -width_2, p.side_height)) + world_coordinate;
    let bh = (rotation * v3(2.0 * width_2, -width_2, p.side_height)) + world_coordinate;
    let ch = (rotation * v3(2.0 * width_2, width_2, p.side_height)) + world_coordinate;
    let dh = (rotation * v3(-width_2, width_2, p.side_height)) + world_coordinate;

    let el = (rotation * v3((2.0 * width_2) + p.bow_length, 0.0, 0.0)) + world_coordinate;
    let eh = (rotation * v3((2.0 * width_2) + p.bow_length, 0.0, p.side_height)) + world_coordinate;

    let sa = (rotation * v3(2.0 * width_2 + (p.bow_length / 2.0), 0.0, p.side_height))
        + world_coordinate;
    let sb = (rotation * v3(2.0 * width_2 + (p.bow_length / 2.0), 0.0, p.mast_height))
        + world_coordinate;
    let sc = (rotation * v3(-0.3 * width_2, 1.5 * width_2, p.side_height)) + world_coordinate;

    let mut floats = vec![];
    floats.append(&mut get_colored_vertices_from_square_both_sides(
        &[al, bl, bh, ah],
        &square_coloring,
    ));
    floats.append(&mut get_colored_vertices_from_square_both_sides(
        &[dl, al, ah, dh],
        &square_coloring,
    ));
    floats.append(&mut get_colored_vertices_from_square_both_sides(
        &[el, bl, bh, eh],
        &square_coloring,
    ));
    floats.append(&mut get_colored_vertices_from_square_both_sides(
        &[dl, cl, ch, dh],
        &square_coloring,
    ));
    floats.append(&mut get_colored_vertices_from_square_both_sides(
        &[cl, el, eh, ch],
        &square_coloring,
    ));
    floats.append(&mut get_colored_vertices_from_square_both_sides(
        &[al, bl, cl, dl],
        &square_coloring,
    ));
    floats.append(&mut get_colored_vertices_from_triangle_both_sides(
        &[bl, el, cl],
        &triangle_coloring,
    ));
    floats.append(&mut get_colored_vertices_from_triangle_both_sides(
        &[bh, eh, ch],
        &triangle_coloring,
    ));

    let sail_coloring = AngleTriangleColoring::new(p.sail_color, p.light_direction);

    floats.append(&mut get_colored_vertices_from_triangle_both_sides(
        &[sa, sb, sc],
        &sail_coloring,
    ));

    vec![Command::UpdateVertices {
        name: name.to_string(),
        index: 0,
        floats,
    }]
}
