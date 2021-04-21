use super::terrain::TerrainGeometry;
use super::utils::*;
use cell_traits::*;
use color::Color;
use commons::barycentric::triangle_interpolate_any;
use commons::grid::Grid;
use commons::*;
use graphics::Drawing;
use Command;

pub const HOUSE_FLOATS: usize = 216;

pub struct House<'a> {
    pub position: &'a V2<usize>,
    pub width: &'a f32,
    pub height: &'a f32,
    pub roof_height: &'a f32,
    pub base_color: &'a Color,
    pub light_direction: &'a V3<f32>,
}

#[allow(clippy::many_single_char_names)]
fn get_floats<T>(terrain: &dyn Grid<T>, house: House) -> Vec<f32>
where
    T: WithPosition + WithElevation + WithVisibility + WithJunction,
{
    let House {
        position,
        width,
        height,
        roof_height,
        base_color,
        light_direction,
    } = house;

    let triangle_coloring = AngleTriangleColoring::new(*base_color, *light_direction);
    let square_coloring = AngleSquareColoring::new(*base_color, *light_direction);

    let x = position.x as f32 + 0.5;
    let y = position.y as f32 + 0.5;
    let w = width;

    let [a, b, c, d] = get_house_base_corners(terrain, &position, w);
    let zs = [a.z, b.z, c.z, d.z];
    let floor_z = zs.iter().max_by(unsafe_ordering).unwrap();

    let e = v3(x - w, y - w, floor_z + height);
    let f = v3(x + w, y - w, floor_z + height);
    let g = v3(x + w, y + w, floor_z + height);
    let h = v3(x - w, y + w, floor_z + height);

    let s = v3(x, y, floor_z + height + roof_height);

    let mut floats = Vec::with_capacity(HOUSE_FLOATS);

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
