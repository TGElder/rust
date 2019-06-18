use super::geometry::*;
use crate::drawing::utils::*;
use cell_traits::*;
use color::Color;
use commons::scale::*;
use commons::*;
use commons::{M, V2, V3};
use std::collections::HashMap;

fn entire_triangle_at_sea_level(triangle: &[V3<f32>; 3], sea_level: f32) -> bool {
    triangle[0].z == sea_level && triangle[1].z == sea_level && triangle[2].z == sea_level
}

pub trait TerrainColoring<T>
where
    T: WithPosition + WithElevation + WithVisibility + WithJunction,
{
    fn color(
        &self,
        terrain: &Grid<T>,
        tile: &V2<usize>,
        triangle: &[V3<f32>; 3],
    ) -> [Option<Color>; 3];
}

pub struct ShadedTileTerrainColoring {
    colors: M<Color>,
    sea_color: Color,
    sea_level: f32,
    shading: Box<SquareColoring>,
}

impl ShadedTileTerrainColoring {
    pub fn new(
        colors: M<Color>,
        sea_color: Color,
        sea_level: f32,
        light_direction: V3<f32>,
    ) -> ShadedTileTerrainColoring {
        ShadedTileTerrainColoring {
            colors,
            sea_color,
            sea_level,
            shading: Box::new(AngleSquareColoring::new(
                Color::new(1.0, 1.0, 1.0, 1.0),
                light_direction,
            )),
        }
    }
}

impl<T> TerrainColoring<T> for ShadedTileTerrainColoring
where
    T: WithPosition + WithElevation + WithVisibility + WithJunction,
{
    fn color(
        &self,
        terrain: &Grid<T>,
        tile: &V2<usize>,
        triangle: &[V3<f32>; 3],
    ) -> [Option<Color>; 3] {
        let color = Some(if entire_triangle_at_sea_level(triangle, self.sea_level) {
            self.sea_color
        } else {
            let geometry = TerrainGeometry::of(terrain);
            let border = geometry.get_border_for_tile(&tile, true);
            let shade = self
                .shading
                .get_colors(&[border[0], border[1], border[2], border[3]])[0];
            self.colors.get_cell(tile).unwrap().mul(&shade)
        });
        [color, color, color]
    }
}

pub struct NodeTerrainColoring {
    colors: M<Option<Color>>,
}

impl NodeTerrainColoring {
    pub fn new(colors: M<Option<Color>>) -> NodeTerrainColoring {
        NodeTerrainColoring { colors }
    }

    pub fn from_data(data: M<f32>) -> NodeTerrainColoring {
        let min = *data.iter().min_by(unsafe_ordering).unwrap();
        let max = *data.iter().max_by(unsafe_ordering).unwrap();
        let scale = Scale::new((min, max), (0.0, 1.0));
        let (width, height) = data.shape();
        let colors = M::from_fn(width, height, |x, y| {
            let shade = scale.scale(data[(x, y)]);
            Some(Color::new(shade, shade, shade, 1.0))
        });
        NodeTerrainColoring { colors }
    }

    fn get_color_for_vertex(&self, vertex: V3<f32>) -> Option<Color> {
        self.colors[(vertex.x.round() as usize, vertex.y.round() as usize)]
    }
}

impl<T> TerrainColoring<T> for NodeTerrainColoring
where
    T: WithPosition + WithElevation + WithVisibility + WithJunction,
{
    fn color(&self, _: &Grid<T>, _: &V2<usize>, triangle: &[V3<f32>; 3]) -> [Option<Color>; 3] {
        [
            self.get_color_for_vertex(triangle[0]),
            self.get_color_for_vertex(triangle[1]),
            self.get_color_for_vertex(triangle[2]),
        ]
    }
}

pub struct Layer<T>
where
    T: WithPosition + WithElevation + WithVisibility + WithJunction,
{
    coloring: Box<TerrainColoring<T> + Send>,
    priority: i64,
}

pub struct LayerColoring<T>
where
    T: WithPosition + WithElevation + WithVisibility + WithJunction,
{
    layers: HashMap<String, Layer<T>>,
    order: Vec<String>,
}

impl<T> LayerColoring<T>
where
    T: WithPosition + WithElevation + WithVisibility + WithJunction,
{
    pub fn new() -> LayerColoring<T> {
        LayerColoring {
            layers: HashMap::new(),
            order: vec![],
        }
    }

    fn update_order(&mut self) {
        let mut order: Vec<String> = self.layers.keys().cloned().collect();
        order.sort_unstable_by(|a, b| self.get_priority(a).cmp(&self.get_priority(b)));
        self.order = order;
    }

    pub fn add_layer(
        &mut self,
        name: String,
        coloring: Box<TerrainColoring<T> + Send>,
        priority: i64,
    ) {
        self.layers.insert(name, Layer { coloring, priority });
        self.update_order();
    }

    pub fn remove_layer(&mut self, name: &str) {
        self.layers.remove(name);
        self.update_order();
    }

    pub fn has_layer(&self, name: &str) -> bool {
        self.layers.contains_key(name)
    }

    pub fn get_priority(&self, name: &str) -> Option<i64> {
        self.layers.get(name).map(|layer| layer.priority)
    }

    pub fn set_priority(&mut self, name: &str, priority: i64) {
        self.layers.get_mut(name).unwrap().priority = priority;
        self.update_order();
    }
}

impl<T> TerrainColoring<T> for LayerColoring<T>
where
    T: WithPosition + WithElevation + WithVisibility + WithJunction,
{
    fn color(
        &self,
        terrain: &Grid<T>,
        tile: &V2<usize>,
        triangle: &[V3<f32>; 3],
    ) -> [Option<Color>; 3] {
        let mut out = [None, None, None];
        for name in self.order.iter() {
            let coloring = self
                .layers
                .get(name)
                .unwrap()
                .coloring
                .color(terrain, tile, triangle);
            for i in 0..3 {
                if out[i] == None {
                    out[i] = coloring[i];
                }
            }
        }
        return out;
    }
}
