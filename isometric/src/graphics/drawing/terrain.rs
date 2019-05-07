use super::super::engine::DrawingType;
use super::super::vertex_objects::{MultiVBO, VBO};
use super::utils::*;
use super::Drawing;
use color::Color;
use coords::WorldCoord;
use terrain::{Edge, Node, Terrain};
use utils::Index2D;
use {v2, M, V2};

pub struct NodeDrawing {
    vbo: VBO,
    z_mod: f32,
}

impl Drawing for NodeDrawing {
    fn draw(&self) {
        self.vbo.draw();
    }

    fn get_z_mod(&self) -> f32 {
        self.z_mod
    }

    fn drawing_type(&self) -> &DrawingType {
        self.vbo.drawing_type()
    }

    fn get_visibility_check_coord(&self) -> Option<&WorldCoord> {
        None
    }
}

impl NodeDrawing {
    pub fn new(terrain: &Terrain, nodes: &Vec<Node>, color: &Color, z_mod: f32) -> NodeDrawing {
        let mut vbo = VBO::new(DrawingType::Plain);

        let mut vertices = vec![];

        for node in nodes {
            for triangle in terrain.get_triangles(Terrain::get_index_for_node(&node)) {
                vertices.append(&mut get_uniform_colored_vertices_from_triangle(
                    &triangle, color,
                ));
            }
        }

        vbo.load(vertices);

        NodeDrawing { vbo, z_mod }
    }
}

pub struct EdgeDrawing {
    vbo: VBO,
    z_mod: f32,
}

impl Drawing for EdgeDrawing {
    fn draw(&self) {
        self.vbo.draw();
    }

    fn get_z_mod(&self) -> f32 {
        self.z_mod
    }

    fn drawing_type(&self) -> &DrawingType {
        self.vbo.drawing_type()
    }

    fn get_visibility_check_coord(&self) -> Option<&WorldCoord> {
        None
    }
}

impl EdgeDrawing {
    pub fn new(terrain: &Terrain, nodes: &Vec<Edge>, color: &Color, z_mod: f32) -> EdgeDrawing {
        let mut vbo = VBO::new(DrawingType::Plain);

        let mut vertices = vec![];

        for node in nodes {
            for triangle in terrain.get_triangles(Terrain::get_index_for_edge(&node)) {
                vertices.append(&mut get_uniform_colored_vertices_from_triangle(
                    &triangle, color,
                ));
            }
        }

        vbo.load(vertices);

        EdgeDrawing { vbo, z_mod }
    }
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
        match self.index.get(position) {
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
    vbo: MultiVBO,
    index: TerrainIndex,
}

impl Drawing for TerrainDrawing {
    fn draw(&self) {
        self.vbo.draw();
    }

    fn get_z_mod(&self) -> f32 {
        0.0
    }

    fn drawing_type(&self) -> &DrawingType {
        self.vbo.drawing_type()
    }

    fn get_visibility_check_coord(&self) -> Option<&WorldCoord> {
        None
    }
}

impl TerrainDrawing {
    pub fn new(width: usize, height: usize, slab_size: usize) -> TerrainDrawing {
        let max_floats_per_index = 9 * // 9 floats per triangle
            2 * // 2 triangles per cell
            slab_size * slab_size * 4; // cells per slab
        let index = TerrainIndex::new(width, height, slab_size);
        let vbo = MultiVBO::new(DrawingType::Plain, index.indices(), max_floats_per_index);
        TerrainDrawing { vbo, index }
    }

    pub fn update(
        &mut self,
        terrain: &Terrain,
        color_matrix: &M<Color>,
        shading: &Box<SquareColoring>,
        from: V2<usize>,
        to: V2<usize>,
    ) {
        let mut vertices = vec![];

        for x in from.x..to.x {
            for y in from.y..to.y {
                let tile_index = v2(x, y);
                let grid_index = Terrain::get_index_for_tile(&tile_index);
                let border = terrain.get_border(grid_index);
                let shade = shading.get_colors(&[border[0], border[1], border[2], border[3]])[0];
                let color = color_matrix[(x, y)].mul(&shade);
                for triangle in terrain.get_triangles_for_tile(&tile_index) {
                    vertices.append(&mut get_uniform_colored_vertices_from_triangle(
                        &triangle, &color,
                    ));
                }
            }
        }

        let index = self.index.get(from).unwrap();
        self.vbo.load(index, vertices);
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
