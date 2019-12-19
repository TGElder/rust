use super::geometry::*;
use crate::drawing::utils::*;
use cell_traits::*;
use color::Color;
use commons::scale::*;
use commons::*;
use commons::{M, V2, V3};
use std::marker::PhantomData;

pub trait TerrainColoring<T>
where
    T: WithPosition + WithElevation + WithVisibility + WithJunction,
{
    fn color(
        &self,
        terrain: &dyn Grid<T>,
        tile: &V2<usize>,
        triangle: &[V3<f32>; 3],
    ) -> [Option<Color>; 3];
}

pub struct ShadedTileTerrainColoring<T, C>
where
    T: WithPosition + WithElevation + WithVisibility + WithJunction,
    C: TerrainColoring<T>,
{
    coloring: C,
    shading: Box<dyn SquareColoring>,
    phantom: PhantomData<T>,
}

impl<T, C> ShadedTileTerrainColoring<T, C>
where
    T: WithPosition + WithElevation + WithVisibility + WithJunction,
    C: TerrainColoring<T>,
{
    pub fn new(coloring: C, light_direction: V3<f32>) -> ShadedTileTerrainColoring<T, C> {
        ShadedTileTerrainColoring {
            coloring,
            shading: Box::new(AngleSquareColoring::new(
                Color::new(1.0, 1.0, 1.0, 1.0),
                light_direction,
            )),
            phantom: PhantomData,
        }
    }
}

impl<T, C> TerrainColoring<T> for ShadedTileTerrainColoring<T, C>
where
    T: WithPosition + WithElevation + WithVisibility + WithJunction,
    C: TerrainColoring<T>,
{
    fn color(
        &self,
        terrain: &dyn Grid<T>,
        tile: &V2<usize>,
        triangle: &[V3<f32>; 3],
    ) -> [Option<Color>; 3] {
        let geometry = TerrainGeometry::of(terrain);
        let border = geometry.get_original_border_for_tile(&tile);
        let shade = self
            .shading
            .get_colors(&[border[0], border[1], border[2], border[3]])[0];
        let base = self.coloring.color(terrain, tile, triangle);
        [
            base[0].map(|color| color * shade),
            base[1].map(|color| color * shade),
            base[2].map(|color| color * shade),
        ]
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
    fn color(&self, _: &dyn Grid<T>, _: &V2<usize>, triangle: &[V3<f32>; 3]) -> [Option<Color>; 3] {
        [
            self.get_color_for_vertex(triangle[0]),
            self.get_color_for_vertex(triangle[1]),
            self.get_color_for_vertex(triangle[2]),
        ]
    }
}

pub struct TileTerrainColoring {
    colors: M<Option<Color>>,
}

impl TileTerrainColoring {
    pub fn new(colors: M<Option<Color>>) -> TileTerrainColoring {
        TileTerrainColoring { colors }
    }

    pub fn from_data(data: M<f32>) -> TileTerrainColoring {
        let min = *data.iter().min_by(unsafe_ordering).unwrap();
        let max = *data.iter().max_by(unsafe_ordering).unwrap();
        let scale = Scale::new((min, max), (0.0, 1.0));
        let (width, height) = data.shape();
        let colors = M::from_fn(width, height, |x, y| {
            let shade = scale.scale(data[(x, y)]);
            Some(Color::new(shade, shade, shade, 1.0))
        });
        TileTerrainColoring { colors }
    }

    fn get_color_for_tile(&self, tile: &V2<usize>) -> Option<Color> {
        self.colors[(tile.x, tile.y)]
    }
}

impl<T> TerrainColoring<T> for TileTerrainColoring
where
    T: WithPosition + WithElevation + WithVisibility + WithJunction,
{
    fn color(&self, _: &dyn Grid<T>, tile: &V2<usize>, _: &[V3<f32>; 3]) -> [Option<Color>; 3] {
        let color = self.get_color_for_tile(tile);
        [color, color, color]
    }
}

pub struct SeaLevelColoring<T, C>
where
    T: WithPosition + WithElevation + WithVisibility + WithJunction,
    C: TerrainColoring<T>,
{
    coloring: C,
    sea_color: Option<Color>,
    sea_level: f32,
    phantom: PhantomData<T>,
}

impl<T, C> SeaLevelColoring<T, C>
where
    T: WithPosition + WithElevation + WithVisibility + WithJunction,
    C: TerrainColoring<T>,
{
    pub fn new(coloring: C, sea_color: Option<Color>, sea_level: f32) -> SeaLevelColoring<T, C> {
        SeaLevelColoring {
            coloring,
            sea_color,
            sea_level,
            phantom: PhantomData,
        }
    }

    fn entire_triangle_at_sea_level(&self, triangle: &[V3<f32>; 3]) -> bool {
        triangle[0].z.almost(self.sea_level)
            && triangle[1].z.almost(self.sea_level)
            && triangle[2].z.almost(self.sea_level)
    }
}

impl<T, C> TerrainColoring<T> for SeaLevelColoring<T, C>
where
    T: WithPosition + WithElevation + WithVisibility + WithJunction,
    C: TerrainColoring<T>,
{
    fn color(
        &self,
        terrain: &dyn Grid<T>,
        tile: &V2<usize>,
        triangle: &[V3<f32>; 3],
    ) -> [Option<Color>; 3] {
        if self.entire_triangle_at_sea_level(triangle) {
            [self.sea_color, self.sea_color, self.sea_color]
        } else {
            self.coloring.color(terrain, tile, triangle)
        }
    }
}

pub struct LayerColoring<T, C, D>
where
    T: WithPosition + WithElevation + WithVisibility + WithJunction,
    C: TerrainColoring<T>,
    D: TerrainColoring<T>,
{
    bottom: C,
    top: D,
    phantom: PhantomData<T>,
}

impl<T, C, D> LayerColoring<T, C, D>
where
    T: WithPosition + WithElevation + WithVisibility + WithJunction,
    C: TerrainColoring<T>,
    D: TerrainColoring<T>,
{
    pub fn new(bottom: C, top: D) -> LayerColoring<T, C, D> {
        LayerColoring {
            bottom,
            top,
            phantom: PhantomData,
        }
    }
}

impl<T, C, D> TerrainColoring<T> for LayerColoring<T, C, D>
where
    T: WithPosition + WithElevation + WithVisibility + WithJunction,
    C: TerrainColoring<T>,
    D: TerrainColoring<T>,
{
    fn color(
        &self,
        terrain: &dyn Grid<T>,
        tile: &V2<usize>,
        triangle: &[V3<f32>; 3],
    ) -> [Option<Color>; 3] {
        let bottom = self.bottom.color(terrain, tile, triangle);
        let top = self.top.color(terrain, tile, triangle);
        let mut out = [None; 3];
        for i in 0..3 {
            out[i] = if let Some(top) = top[i] {
                if let Some(bottom) = bottom[i] {
                    Some(top.layer_over(&bottom))
                } else {
                    None
                }
            } else {
                bottom[i]
            }
        }
        out
    }
}
