use super::utils::*;
use crate::graphics::Drawing;
use crate::Command;
use color::Color;
use commons::*;
use coords::*;

pub struct DrawHouseParams {
    pub width: f32,
    pub height: f32,
    pub roof_height: f32,
    pub basement_z: f32,
    pub base_color: Color,
    pub light_direction: V3<f32>,
}

pub fn draw_house(name: String, world_coordinate: WorldCoord, p: &DrawHouseParams) -> Vec<Command> {
    let triangle_coloring = AngleTriangleColoring::new(p.base_color, p.light_direction);
    let square_coloring = AngleSquareColoring::new(p.base_color, p.light_direction);

    let x = world_coordinate.x as f32;
    let y = world_coordinate.y as f32;
    let z = world_coordinate.z as f32;

    let w = p.width;

    let a = v3(x - w, y - w, p.basement_z);
    let b = v3(x + w, y - w, p.basement_z);
    let c = v3(x + w, y + w, p.basement_z);
    let d = v3(x - w, y + w, p.basement_z);
    let e = v3(x - w, y - w, z + p.height);
    let f = v3(x + w, y - w, z + p.height);
    let g = v3(x + w, y + w, z + p.height);
    let h = v3(x - w, y + w, z + p.height);

    let s = v3(x, y, z + p.height + p.roof_height);

    let mut floats = vec![];
    floats.append(&mut get_colored_vertices_from_square(
        &[e, h, d, a],
        &square_coloring,
    ));
    floats.append(&mut get_colored_vertices_from_square(
        &[h, g, c, d],
        &square_coloring,
    ));
    floats.append(&mut get_colored_vertices_from_square(
        &[g, f, b, c],
        &square_coloring,
    ));
    floats.append(&mut get_colored_vertices_from_square(
        &[f, e, a, b],
        &square_coloring,
    ));
    floats.append(&mut get_colored_vertices_from_triangle(
        &[h, e, s],
        &triangle_coloring,
    ));
    floats.append(&mut get_colored_vertices_from_triangle(
        &[g, h, s],
        &triangle_coloring,
    ));
    floats.append(&mut get_colored_vertices_from_triangle(
        &[f, g, s],
        &triangle_coloring,
    ));
    floats.append(&mut get_colored_vertices_from_triangle(
        &[e, f, s],
        &triangle_coloring,
    ));

    vec![
        Command::CreateDrawing(Drawing::plain(name.clone(), floats.len())),
        Command::UpdateDrawing {
            name,
            index: 0,
            floats,
        },
    ]
}
