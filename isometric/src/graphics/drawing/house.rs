use super::terrain::TerrainGeometry;
use super::utils::*;
use cell_traits::*;
use color::Color;
use commons::barycentric::triangle_interpolate_any;
use commons::grid::Grid;
use commons::*;
use graphics::Drawing;
use Command;

pub const HOUSE_FLOATS: usize = 252;

pub struct House<'a> {
    pub position: &'a V2<usize>,
    pub width: &'a f32,
    pub height: &'a f32,
    pub roof_height: &'a f32,
    pub rotated: bool,
    pub base_color: &'a Color,
    pub light_direction: &'a V3<f32>,
}

fn get_floats<T>(terrain: &dyn Grid<T>, house: House) -> Vec<f32>
where
    T: WithPosition + WithElevation + WithVisibility + WithJunction,
{
    let House {
        position,
        width,
        height,
        roof_height,
        rotated,
        base_color,
        light_direction,
    } = house;

    let triangle_coloring = AngleTriangleColoring::new(*base_color, *light_direction);
    let square_coloring = AngleSquareColoring::new(*base_color, *light_direction);

    let x = position.x as f32 + 0.5;
    let y = position.y as f32 + 0.5;
    let w = width;

    let [base_a, base_b, base_c, base_d] = get_house_base_corners(terrain, position, w);
    let zs = [base_a.z, base_b.z, base_c.z, base_d.z];
    let floor_z = zs.iter().max_by(unsafe_ordering).unwrap();

    let top_a = v3(x - w, y - w, floor_z + height);
    let top_b = v3(x + w, y - w, floor_z + height);
    let top_c = v3(x + w, y + w, floor_z + height);
    let top_d = v3(x - w, y + w, floor_z + height);

    let (roof_a, roof_b, roof_c, roof_d, ridge_a, ridge_b) = if rotated {
        (
            top_a,
            top_b,
            top_c,
            top_d,
            v3(x - w, y, floor_z + height + roof_height),
            v3(x + w, y, floor_z + height + roof_height),
        )
    } else {
        (
            top_b,
            top_c,
            top_d,
            top_a,
            v3(x, y - w, floor_z + height + roof_height),
            v3(x, y + w, floor_z + height + roof_height),
        )
    };

    let mut floats = Vec::with_capacity(HOUSE_FLOATS);

    floats.append(&mut get_colored_vertices_from_square(
        &[top_a, top_d, base_d, base_a],
        &square_coloring,
    ));
    floats.append(&mut get_colored_vertices_from_square(
        &[top_d, top_c, base_c, base_d],
        &square_coloring,
    ));
    floats.append(&mut get_colored_vertices_from_square(
        &[top_c, top_b, base_b, base_c],
        &square_coloring,
    ));
    floats.append(&mut get_colored_vertices_from_square(
        &[top_b, top_a, base_a, base_b],
        &square_coloring,
    ));

    floats.append(&mut get_colored_vertices_from_triangle(
        &[roof_d, roof_a, ridge_a],
        &triangle_coloring,
    ));
    floats.append(&mut get_colored_vertices_from_square(
        &[roof_c, roof_d, ridge_a, ridge_b],
        &square_coloring,
    ));
    floats.append(&mut get_colored_vertices_from_triangle(
        &[roof_b, roof_c, ridge_b],
        &triangle_coloring,
    ));
    floats.append(&mut get_colored_vertices_from_square(
        &[roof_a, roof_b, ridge_b, ridge_a],
        &square_coloring,
    ));

    floats
}

pub fn get_house_base_corners<T>(
    terrain: &dyn Grid<T>,
    position: &V2<usize>,
    width: &f32,
) -> [V3<f32>; 4]
where
    T: WithPosition + WithElevation + WithVisibility + WithJunction,
{
    let x = position.x as f32 + 0.5;
    let y = position.y as f32 + 0.5;

    let geometry = TerrainGeometry::of(terrain);
    let triangles = geometry.get_triangles_for_tile(position);
    let get_base_corner = move |offset: V2<usize>| {
        let corner2d = v2(
            x + (offset.x as f32 * 2.0 - 1.0) * width,
            y + (offset.y as f32 * 2.0 - 1.0) * width,
        );
        v3(
            corner2d.x,
            corner2d.y,
            triangle_interpolate_any(&corner2d, &triangles)
                .unwrap_or_else(|| terrain.get_cell_unsafe(&(position + offset)).elevation()),
        )
    };

    [
        get_base_corner(v2(0, 0)),
        get_base_corner(v2(1, 0)),
        get_base_corner(v2(1, 1)),
        get_base_corner(v2(0, 1)),
    ]
}

pub fn create_house_drawing(name: String, count: usize) -> Command {
    Command::CreateDrawing(Drawing::plain(name, HOUSE_FLOATS * count))
}

pub fn update_house_drawing_vertices<T>(
    name: String,
    terrain: &dyn Grid<T>,
    houses: Vec<House>,
) -> Command
where
    T: WithPosition + WithElevation + WithVisibility + WithJunction,
{
    let mut floats = Vec::with_capacity(HOUSE_FLOATS * houses.len());

    for house in houses {
        floats.append(&mut get_floats(terrain, house));
    }

    Command::UpdateVertices {
        name,
        index: 0,
        floats,
    }
}

pub fn create_and_update_house_drawing<T>(
    name: String,
    terrain: &dyn Grid<T>,
    houses: Vec<House>,
) -> Vec<Command>
where
    T: WithPosition + WithElevation + WithVisibility + WithJunction,
{
    vec![
        create_house_drawing(name.clone(), houses.len()),
        update_house_drawing_vertices(name, terrain, houses),
    ]
}
