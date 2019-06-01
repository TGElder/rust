use super::utils::*;
use crate::graphics::Drawing;
use crate::Command;
use color::Color;
use coords::*;

pub fn draw_house(
    name: String,
    world_coordinate: WorldCoord,
    width: f32,
    height: f32,
    roof_height: f32,
    basement_z: f32,
    base_color: Color,
    light_direction: na::Vector3<f32>,
) -> Vec<Command> {
    let triangle_coloring: Box<TriangleColoring> =
        Box::new(AngleTriangleColoring::new(base_color, light_direction));
    let square_coloring: Box<SquareColoring> =
        Box::new(AngleSquareColoring::new(base_color, light_direction));

    let x = world_coordinate.x as f32;
    let y = world_coordinate.y as f32;
    let z = world_coordinate.z as f32;

    let a = na::Vector3::new(x - width, y - width, basement_z);
    let b = na::Vector3::new(x + width, y - width, basement_z);
    let c = na::Vector3::new(x + width, y + width, basement_z);
    let d = na::Vector3::new(x - width, y + width, basement_z);
    let e = na::Vector3::new(x - width, y - width, z + height);
    let f = na::Vector3::new(x + width, y - width, z + height);
    let g = na::Vector3::new(x + width, y + width, z + height);
    let h = na::Vector3::new(x - width, y + width, z + height);

    let s = na::Vector3::new(x, y, z + height + roof_height);

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
