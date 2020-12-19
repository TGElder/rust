use super::terrain::TerrainGeometry;
use super::utils::*;
use cell_traits::*;
use color::Color;
use commons::barycentric::triangle_interpolate_any;
use commons::grid::Grid;
use commons::*;
use graphics::Drawing;
use Command;

pub struct DrawHouseParams {
    pub width: f32,
    pub height: f32,
    pub roof_height: f32,
    pub base_color: Color,
    pub light_direction: V3<f32>,
}

#[allow(clippy::many_single_char_names)]
pub fn draw_house<T>(
    name: String,
    terrain: &dyn Grid<T>,
    position: &V2<usize>,
    p: &DrawHouseParams,
) -> Vec<Command>
where
    T: WithPosition + WithElevation + WithVisibility + WithJunction,
{
    let triangle_coloring = AngleTriangleColoring::new(p.base_color, p.light_direction);
    let square_coloring = AngleSquareColoring::new(p.base_color, p.light_direction);

    let x = position.x as f32 + 0.5;
    let y = position.y as f32 + 0.5;
    let w = p.width;

    let [a, b, c, d] = get_house_base_corners(terrain, position, w);
    let zs = [a.z, b.z, c.z, d.z];
    let floor_z = zs.iter().max_by(unsafe_ordering).unwrap();

    let e = v3(x - w, y - w, floor_z + p.height);
    let f = v3(x + w, y - w, floor_z + p.height);
    let g = v3(x + w, y + w, floor_z + p.height);
    let h = v3(x - w, y + w, floor_z + p.height);

    let s = v3(x, y, floor_z + p.height + p.roof_height);

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
        Command::UpdateVertices {
            name,
            index: 0,
            floats,
        },
    ]
}

pub fn get_house_base_corners<T>(
    terrain: &dyn Grid<T>,
    position: &V2<usize>,
    width: f32,
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
