use super::geometry::*;
use crate::drawing::utils::*;
use cell_traits::*;
use color::Color;
use commons::almost::Almost;
use commons::grid::Grid;
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

pub struct ShadedTileTerrainColoring<'a, T>
where
    T: WithPosition + WithElevation + WithVisibility + WithJunction,
{
    coloring: Box<dyn TerrainColoring<T> + 'a>,
    shading: Box<dyn SquareColoring>,
    phantom: PhantomData<T>,
}

impl<'a, T> ShadedTileTerrainColoring<'a, T>
where
    T: WithPosition + WithElevation + WithVisibility + WithJunction,
{
    pub fn new(
        coloring: Box<dyn TerrainColoring<T> + 'a>,
        light_direction: V3<f32>,
    ) -> ShadedTileTerrainColoring<'a, T> {
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

impl<'a, T> TerrainColoring<T> for ShadedTileTerrainColoring<'a, T>
where
    T: WithPosition + WithElevation + WithVisibility + WithJunction,
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

pub struct SeaLevelColoring<'a, T>
where
    T: WithPosition + WithElevation + WithVisibility + WithJunction,
{
    coloring: Box<dyn TerrainColoring<T> + 'a>,
    sea_colors: Option<SeaColors>,
    sea_level: f32,
    phantom: PhantomData<T>,
}

pub struct SeaColors {
    pub shallow: Color,
    pub deep: Color,
}

impl<'a, T> SeaLevelColoring<'a, T>
where
    T: WithPosition + WithElevation + WithVisibility + WithJunction,
{
    pub fn new(
        coloring: Box<dyn TerrainColoring<T> + 'a>,
        sea_colors: Option<SeaColors>,
        sea_level: f32,
    ) -> SeaLevelColoring<'a, T> {
        SeaLevelColoring {
            coloring,
            sea_colors,
            sea_level,
            phantom: PhantomData,
        }
    }

    fn entire_triangle_at_sea_level(&self, triangle: &[V3<f32>; 3]) -> bool {
        triangle[0].z.almost(&self.sea_level)
            && triangle[1].z.almost(&self.sea_level)
            && triangle[2].z.almost(&self.sea_level)
    }

    fn sea_color(&self, terrain: &dyn Grid<T>, tile: &V2<usize>) -> Option<Color> {
        let depth_pc = self.min_tile_elevation(terrain, tile) / self.sea_level;
        self.sea_colors
            .as_ref()
            .map(|SeaColors { shallow, deep }| shallow.blend(depth_pc, deep))
    }

    fn min_tile_elevation(&self, terrain: &dyn Grid<T>, tile: &V2<usize>) -> f32 {
        terrain
            .get_corners_in_bounds(tile)
            .into_iter()
            .map(|corner| terrain.get_cell_unsafe(&corner).elevation())
            .max_by(unsafe_ordering)
            .unwrap()
    }
}

impl<'a, T> TerrainColoring<T> for SeaLevelColoring<'a, T>
where
    T: WithPosition + WithElevation + WithVisibility + WithJunction,
{
    fn color(
        &self,
        terrain: &dyn Grid<T>,
        tile: &V2<usize>,
        triangle: &[V3<f32>; 3],
    ) -> [Option<Color>; 3] {
        let color = self.sea_color(terrain, tile);

        if self.entire_triangle_at_sea_level(triangle) {
            [color, color, color]
        } else {
            self.coloring.color(terrain, tile, triangle)
        }
    }
}

pub struct LayerColoring<'a, T>
where
    T: WithPosition + WithElevation + WithVisibility + WithJunction,
{
    layers: Vec<Box<dyn TerrainColoring<T> + 'a>>,
    phantom: PhantomData<T>,
}

impl<'a, T> LayerColoring<'a, T>
where
    T: WithPosition + WithElevation + WithVisibility + WithJunction,
{
    pub fn new(layers: Vec<Box<dyn TerrainColoring<T> + 'a>>) -> LayerColoring<'a, T> {
        LayerColoring {
            layers,
            phantom: PhantomData,
        }
    }
}

impl<'a, T> TerrainColoring<T> for LayerColoring<'a, T>
where
    T: WithPosition + WithElevation + WithVisibility + WithJunction,
{
    fn color(
        &self,
        terrain: &dyn Grid<T>,
        tile: &V2<usize>,
        triangle: &[V3<f32>; 3],
    ) -> [Option<Color>; 3] {
        let mut out =
            unwrap_or!(self.layers.first(), return [None; 3]).color(terrain, tile, triangle);
        for layer in self.layers.iter().skip(1) {
            let colors = layer.color(terrain, tile, triangle);
            for i in 0..3 {
                if let Some(top) = colors[i] {
                    if let Some(bottom) = out[i] {
                        out[i] = Some(top.layer_over(&bottom));
                    }
                }
            }
        }
        out
    }
}

#[derive(Default)]
pub struct NoneColoring<T>
where
    T: WithPosition + WithElevation + WithVisibility + WithJunction,
{
    phantom: PhantomData<T>,
}

impl<T> NoneColoring<T>
where
    T: WithPosition + WithElevation + WithVisibility + WithJunction,
{
    pub fn new() -> NoneColoring<T> {
        NoneColoring {
            phantom: PhantomData,
        }
    }
}

impl<T> TerrainColoring<T> for NoneColoring<T>
where
    T: WithPosition + WithElevation + WithVisibility + WithJunction,
{
    fn color(&self, _: &dyn Grid<T>, _: &V2<usize>, _: &[V3<f32>; 3]) -> [Option<Color>; 3] {
        [None, None, None]
    }
}

impl<'a, T, U> TerrainColoring<T> for &'a U
where
    T: WithPosition + WithElevation + WithVisibility + WithJunction,
    U: TerrainColoring<T>,
{
    fn color(
        &self,
        terrain: &dyn Grid<T>,
        tile: &V2<usize>,
        triangle: &[V3<f32>; 3],
    ) -> [Option<Color>; 3] {
        U::color(self, terrain, tile, triangle)
    }
}

impl<'a, T, U> TerrainColoring<T> for Option<U>
where
    T: WithPosition + WithElevation + WithVisibility + WithJunction,
    U: TerrainColoring<T>,
{
    fn color(
        &self,
        terrain: &dyn Grid<T>,
        tile: &V2<usize>,
        triangle: &[V3<f32>; 3],
    ) -> [Option<Color>; 3] {
        self.as_ref()
            .map(|coloring| coloring.color(terrain, tile, triangle))
            .unwrap_or([None, None, None])
    }
}
