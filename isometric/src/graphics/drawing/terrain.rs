use super::utils::*;
use crate::graphics::Drawing;
use crate::Command;
use color::Color;
use commons::index2d::*;
use commons::{v2, M, V2, V3};
use terrain::{Edge, Node, Terrain};

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

fn entire_triangle_at_sea_level(triangle: [V3<f32>; 3], sea_level: f32) -> bool {
    triangle[0].z == sea_level && triangle[1].z == sea_level && triangle[2].z == sea_level
}

pub fn draw_nodes(
    name: String,
    terrain: &Terrain,
    nodes: &Vec<Node>,
    color: &Color,
    sea_level: f32,
) -> Vec<Command> {
    let mut floats = vec![];

    for node in nodes {
        for triangle in terrain.get_triangles(Terrain::get_index_for_node(&node)) {
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

pub fn draw_edges(
    name: String,
    terrain: &Terrain,
    nodes: &Vec<Edge>,
    color: &Color,
    sea_level: f32,
) -> Vec<Command> {
    let mut floats = vec![];

    for node in nodes {
        for triangle in terrain.get_triangles(Terrain::get_index_for_edge(&node)) {
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
}

impl TerrainDrawing {
    pub fn new(name: String, width: usize, height: usize, slab_size: usize) -> TerrainDrawing {
        let index = TerrainIndex::new(width, height, slab_size);
        let max_floats_per_index = 9 * // 9 floats per triangle
            2 * // 2 triangles per cell
            slab_size * slab_size * 4; // cells per slab
        TerrainDrawing {
            name,
            index,
            max_floats_per_index,
        }
    }

    pub fn init(&self) -> Vec<Command> {
        vec![Command::CreateDrawing(Drawing::multi(
            self.name.clone(),
            self.index.indices(),
            self.max_floats_per_index,
        ))]
    }

    pub fn update(
        &mut self,
        terrain: &Terrain,
        color_matrix: &M<Color>,
        sea_level: f32,
        sea_color: &Color,
        shading: &Box<SquareColoring>,
        from: V2<usize>,
        to: V2<usize>,
    ) -> Vec<Command> {
        let mut floats = vec![];

        for x in from.x..to.x {
            for y in from.y..to.y {
                let tile = v2(x, y);
                let grid_index = Terrain::get_index_for_tile(&tile);
                let border = terrain.get_border(grid_index, true);
                let shade = shading.get_colors(&[border[0], border[1], border[2], border[3]])[0];
                let color = &color_matrix[(tile.x, tile.y)].mul(&shade);
                for triangle in terrain.get_triangles_for_tile(&tile) {
                    let triangle = clip_triangle_to_sea_level(triangle, sea_level);
                    let color = if entire_triangle_at_sea_level(triangle, sea_level) {
                        sea_color
                    } else {
                        color
                    };
                    floats.append(&mut get_uniform_colored_vertices_from_triangle(
                        &triangle, &color,
                    ));
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
