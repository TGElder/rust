mod coloring;
mod geometry;

pub use self::coloring::*;
use self::geometry::*;

use super::utils::*;
use crate::graphics::Drawing;
use crate::Command;
use cell_traits::*;
use color::Color;
use commons::edge::*;
use commons::index2d::*;
use commons::*;

fn clip_to_sea_level(mut vertex: V3<f32>, sea_level: f32) -> V3<f32> {
    vertex.z = vertex.z.max(sea_level);
    vertex
}

fn clip_triangle_to_sea_level(triangle: [V3<f32>; 3], sea_level: f32) -> [V3<f32>; 3] {
    [
        clip_to_sea_level(triangle[0], sea_level),
        clip_to_sea_level(triangle[1], sea_level),
        clip_to_sea_level(triangle[2], sea_level),
    ]
}

pub fn draw_nodes<T>(
    name: String,
    terrain: &dyn Grid<T>,
    nodes: &[V2<usize>],
    color: &Color,
    sea_level: f32,
) -> Vec<Command>
where
    T: WithPosition + WithElevation + WithVisibility + WithJunction,
{
    let mut floats = vec![];
    let geometry = TerrainGeometry::of(terrain);

    for node in nodes {
        for triangle in geometry.get_triangles_for_node(node) {
            floats.append(&mut get_uniform_colored_vertices_from_triangle(
                &clip_triangle_to_sea_level(triangle, sea_level),
                color,
            ));
        }
    }

    vec![
        Command::CreateDrawing(Drawing::plain(name.clone(), floats.len())),
        Command::UpdateDrawing {
            name,
            index: 0,
            floats,
        },
    ]
}

pub fn draw_edges<T>(
    name: String,
    terrain: &dyn Grid<T>,
    edges: &[Edge],
    color: &Color,
    sea_level: f32,
) -> Vec<Command>
where
    T: WithPosition + WithElevation + WithVisibility + WithJunction,
{
    let mut floats = vec![];

    let geometry = TerrainGeometry::of(terrain);

    for edge in edges {
        let triangles = if edge.horizontal() {
            geometry.get_triangles_for_horizontal_edge(&edge.from())
        } else {
            geometry.get_triangles_for_vertical_edge(&edge.from())
        };
        for triangle in triangles {
            floats.append(&mut get_uniform_colored_vertices_from_triangle(
                &clip_triangle_to_sea_level(triangle, sea_level),
                color,
            ));
        }
    }

    vec![
        Command::CreateDrawing(Drawing::plain(name.clone(), floats.len())),
        Command::UpdateDrawing {
            name,
            index: 0,
            floats,
        },
    ]
}

pub struct TexturedTile {
    pub tile: V2<usize>,
    pub rotation: f32,
}

pub fn textured_tiles<T>(
    name: String,
    terrain: &dyn Grid<T>,
    sea_level: f32,
    textured_tiles: &[TexturedTile],
    coloring: &dyn TerrainColoring<T>,
    texture: String,
) -> Vec<Command>
where
    T: WithPosition + WithElevation + WithVisibility + WithJunction,
{
    let mut floats = vec![];
    let geometry = TerrainGeometry::of(terrain);

    for textured_tile in textured_tiles {
        let tile = textured_tile.tile;
        for triangle in geometry.get_triangles_for_tile(&tile) {
            let triangle = clip_triangle_to_sea_level(triangle, sea_level);
            let colors = coloring.color(terrain, &tile, &triangle);
            let colors = [
                colors[0].unwrap_or_else(Color::transparent),
                colors[1].unwrap_or_else(Color::transparent),
                colors[2].unwrap_or_else(Color::transparent),
            ];
            let texture_from = v2(tile.x as f32, tile.y as f32);
            let texture_to = texture_from + v2(1.0, 1.0);
            let texture_coordinates = get_texture_coordinates(
                &triangle,
                texture_from,
                texture_to,
                textured_tile.rotation,
            );
            floats.append(&mut get_textured_vertices_from_triangle(
                &triangle,
                &colors,
                &texture_coordinates,
            ));
        }
    }

    vec![
        Command::CreateDrawing(Drawing::textured(name.clone(), floats.len(), texture)),
        Command::UpdateDrawing {
            name,
            index: 0,
            floats,
        },
    ]
}

#[derive(Debug, PartialEq, Clone, Copy)]
struct TerrainIndex {
    slab_size: usize,
    index: Index2D,
}

#[derive(Debug, PartialEq)]
struct TerrainIndexOutOfBounds {
    slab: V2<usize>,
    index: TerrainIndex,
}

impl TerrainIndex {
    pub fn new(width: usize, height: usize, slab_size: usize) -> TerrainIndex {
        TerrainIndex {
            slab_size,
            index: Index2D::new(width / slab_size, height / slab_size),
        }
    }

    pub fn get(&self, slab: V2<usize>) -> Result<usize, TerrainIndexOutOfBounds> {
        let position = slab / self.slab_size;
        match self.index.get_index(&position) {
            Err(_) => Err(TerrainIndexOutOfBounds { slab, index: *self }),
            Ok(index) => Ok(index),
        }
    }

    pub fn indices(&self) -> usize {
        self.index.indices()
    }
}

#[derive(Clone)]
pub struct TerrainDrawing {
    name: String,
    index: TerrainIndex,
    max_floats_per_index: usize,
    default_color: Color,
}

impl TerrainDrawing {
    pub fn new(name: String, width: usize, height: usize, slab_size: usize) -> TerrainDrawing {
        let index = TerrainIndex::new(width, height, slab_size);
        let max_floats_per_index = 18 * // 18 floats per colored triangle
            4 * // 4 triangles per cell
            slab_size * slab_size; // cells per slab
        TerrainDrawing {
            name,
            index,
            max_floats_per_index,
            default_color: Color::new(0.0, 0.0, 0.0, 0.0),
        }
    }

    pub fn init(&self) -> Vec<Command> {
        vec![Command::CreateDrawing(Drawing::multi(
            self.name.clone(),
            self.index.indices(),
            self.max_floats_per_index,
        ))]
    }

    pub fn update<T>(
        &mut self,
        terrain: &dyn Grid<T>,
        sea_level: f32,
        coloring: &dyn TerrainColoring<T>,
        from: V2<usize>,
        to: V2<usize>,
    ) -> Vec<Command>
    where
        T: WithPosition + WithElevation + WithVisibility + WithJunction,
    {
        let mut floats = vec![];

        let geometry = TerrainGeometry::of(terrain);

        for x in from.x..to.x {
            for y in from.y..to.y {
                let tile = v2(x, y);
                for triangle in geometry.get_triangles_for_tile(&tile) {
                    let triangle = clip_triangle_to_sea_level(triangle, sea_level);
                    if let [Some(a), Some(b), Some(c)] = coloring.color(terrain, &tile, &triangle) {
                        let colors = [a, b, c];
                        floats.append(&mut get_specific_colored_vertices_from_triangle(
                            &triangle, &colors,
                        ));
                    }
                }
            }
        }

        let index = self.index.get(from).unwrap();
        vec![Command::UpdateDrawing {
            name: self.name.clone(),
            index,
            floats,
        }]
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_terrain_index_indices() {
        let index = TerrainIndex::new(128, 64, 32);
        assert_eq!(index.indices(), 8);
    }

    #[test]
    fn test_terrain_index_get_index() {
        let index = TerrainIndex::new(128, 64, 32);
        assert_eq!(index.get(v2(0, 0)).unwrap(), 0);
        assert_eq!(index.get(v2(32, 0)).unwrap(), 1);
        assert_eq!(index.get(v2(64, 0)).unwrap(), 2);
        assert_eq!(index.get(v2(96, 0)).unwrap(), 3);
        assert_eq!(index.get(v2(0, 32)).unwrap(), 4);
        assert_eq!(index.get(v2(32, 32)).unwrap(), 5);
        assert_eq!(index.get(v2(64, 32)).unwrap(), 6);
        assert_eq!(index.get(v2(96, 32)).unwrap(), 7);
    }

    #[test]
    fn test_terrain_index_x_out_of_bounds() {
        let index = TerrainIndex::new(128, 64, 32);
        assert_eq!(
            index.get(v2(128, 0)),
            Err(TerrainIndexOutOfBounds {
                slab: v2(128, 0),
                index,
            })
        );
    }

    #[test]
    fn test_terrain_index_y_out_of_bounds() {
        let index = TerrainIndex::new(128, 64, 32);
        assert_eq!(
            index.get(v2(0, 64)),
            Err(TerrainIndexOutOfBounds {
                slab: v2(0, 64),
                index,
            })
        );
    }
}
