use crate::artists::WorldColoring;
use crate::world::{World, WorldCell, WorldObject};
use commons::grid::Grid;
use commons::{v2, M, V2, V3};
use isometric::drawing::{
    LayerColoring, SeaLevelColoring, ShadedTileTerrainColoring, TerrainColoring,
};
use isometric::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorldColoringParameters {
    pub colors: BaseColors,
    pub beach_level: f32,
    pub cliff_gradient: f32,
    pub snow_temperature: f32,
    pub light_direction: V3<f32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct BaseColors {
    sea: Color,
    cliff: Color,
    beach: Color,
    desert: Color,
    vegetation: Color,
    snow: Color,
}

impl Default for BaseColors {
    fn default() -> BaseColors {
        BaseColors {
            sea: Color::new(0.0, 0.0, 1.0, 1.0),
            cliff: Color::new(0.5, 0.4, 0.3, 1.0),
            beach: Color::new(1.0, 1.0, 0.0, 1.0),
            desert: Color::new(1.0, 0.8, 0.6, 1.0),
            vegetation: Color::new(0.0, 1.0, 0.0, 1.0),
            snow: Color::new(1.0, 1.0, 1.0, 1.0),
        }
    }
}

pub fn world_coloring<'a>(
    world: &'a World,
    params: &'a WorldColoringParameters,
    overlay: &'a Option<Overlay>,
) -> WorldColoring<'a> {
    WorldColoring {
        terrain: terrain(world, params, overlay),
        crops: crops(world, overlay),
    }
}

fn terrain<'a>(
    world: &'a World,
    params: &'a WorldColoringParameters,
    overlay: &'a Option<Overlay>,
) -> Box<dyn TerrainColoring<WorldCell> + 'a> {
    let base = Box::new(BaseColoring::new(&params, world));
    let shaded = Box::new(ShadedTileTerrainColoring::new(base, params.light_direction));
    let with_overlay: Box<dyn TerrainColoring<WorldCell>> =
        Box::new(LayerColoring::new(vec![shaded, Box::new(overlay)]));
    Box::new(SeaLevelColoring::new(
        with_overlay,
        Some(params.colors.sea),
        world.sea_level(),
    ))
}

fn crops<'a>(
    world: &'a World,
    overlay: &'a Option<Overlay>,
) -> Box<dyn TerrainColoring<WorldCell> + 'a> {
    Box::new(SeaLevelColoring::new(
        Box::new(overlay),
        None,
        world.sea_level(),
    ))
}

pub struct BaseColoring<'a> {
    params: &'a WorldColoringParameters,
    world: &'a World,
}

impl<'a> BaseColoring<'a> {
    fn new(params: &'a WorldColoringParameters, world: &'a World) -> BaseColoring<'a> {
        BaseColoring { params, world }
    }

    fn get_groundwater(world: &World, position: &V2<usize>) -> f32 {
        world.tile_avg_groundwater(position).unwrap()
    }

    fn get_color(&self, world: &World, position: &V2<usize>) -> Color {
        let beach_level = self.params.beach_level;
        let cliff_gradient = self.params.cliff_gradient;
        let snow_temperature = self.params.snow_temperature;
        let max_gradient = world.get_max_abs_rise(&position);
        let min_elevation = world.get_lowest_corner(&position);
        if max_gradient > cliff_gradient {
            self.params.colors.cliff
        } else if world
            .tile_avg_temperature(position)
            .map(|temperature| temperature < snow_temperature)
            .unwrap_or_default()
        {
            self.params.colors.snow
        } else if min_elevation <= beach_level {
            self.params.colors.beach
        } else {
            let groundwater = Self::get_groundwater(&world, &position);
            self.params
                .colors
                .vegetation
                .blend(groundwater, &self.params.colors.desert)
        }
    }
}

impl<'a> TerrainColoring<WorldCell> for BaseColoring<'a> {
    fn color(
        &self,
        _: &dyn Grid<WorldCell>,
        tile: &V2<usize>,
        _: &[V3<f32>; 3],
    ) -> [Option<Color>; 3] {
        if let WorldObject::Crop { .. } = self.world.get_cell_unsafe(tile).object {
            [None, None, None]
        } else {
            let color = Some(self.get_color(&self.world, tile));
            [color, color, color]
        }
    }
}

#[derive(Clone)]
pub struct Overlay<'a> {
    pub from: V2<usize>,
    pub colors: &'a M<Option<Color>>,
}

impl<'a> TerrainColoring<WorldCell> for Overlay<'a> {
    fn color(
        &self,
        _: &dyn Grid<WorldCell>,
        tile: &V2<usize>,
        _: &[V3<f32>; 3],
    ) -> [Option<Color>; 3] {
        let position = v2(tile.x - self.from.x, tile.y - self.from.y);
        let color = *self.colors.get_cell_unsafe(&position);
        [color, color, color]
    }
}
