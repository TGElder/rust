use crate::artists::WorldColoring;
use crate::game::GameState;
use crate::nation::Nation;
use crate::settlement::Settlement;
use crate::territory::Claim;
use crate::world::{World, WorldCell, WorldObject};
use commons::{Grid, V2, V3};
use isometric::drawing::{
    LayerColoring, NoneColoring, SeaLevelColoring, ShadedTileTerrainColoring, TerrainColoring,
};
use isometric::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct WorldColoringParameters {
    pub base: BaseColoringParameters,
    pub territory: TerritoryColoringParameters,
}

pub fn world_coloring(game_state: &GameState, territory_layer: bool) -> WorldColoring {
    WorldColoring {
        terrain: terrain(game_state, territory_layer),
        crops: crops(game_state, territory_layer),
    }
}

fn terrain<'a>(
    game_state: &'a GameState,
    territory_layer: bool,
) -> Box<dyn TerrainColoring<WorldCell> + 'a> {
    let params = &game_state.params.world_coloring;
    let base = Box::new(BaseColoring::new(&params.base, game_state));
    let shaded = Box::new(ShadedTileTerrainColoring::new(
        base,
        game_state.params.light_direction,
    ));
    let coloring: Box<dyn TerrainColoring<WorldCell>> = if territory_layer {
        let territory = Box::new(TerritoryColoring::new(&params.territory, game_state));
        Box::new(LayerColoring::new(vec![shaded, territory]))
    } else {
        shaded
    };
    Box::new(SeaLevelColoring::new(
        coloring,
        Some(params.base.sea),
        game_state.params.world_gen.sea_level as f32,
    ))
}

fn crops<'a>(
    game_state: &'a GameState,
    territory_layer: bool,
) -> Box<dyn TerrainColoring<WorldCell> + 'a> {
    if territory_layer {
        let params = &game_state.params.world_coloring;
        let territory = Box::new(TerritoryColoring::new(&params.territory, game_state));
        Box::new(SeaLevelColoring::new(
            territory,
            None,
            game_state.params.world_gen.sea_level as f32,
        ))
    } else {
        Box::new(NoneColoring::new())
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct BaseColoringParameters {
    sea: Color,
    cliff: Color,
    beach: Color,
    desert: Color,
    vegetation: Color,
    snow: Color,
}

impl Default for BaseColoringParameters {
    fn default() -> BaseColoringParameters {
        BaseColoringParameters {
            sea: Color::new(0.0, 0.0, 1.0, 1.0),
            cliff: Color::new(0.5, 0.4, 0.3, 1.0),
            beach: Color::new(1.0, 1.0, 0.0, 1.0),
            desert: Color::new(1.0, 0.8, 0.6, 1.0),
            vegetation: Color::new(0.0, 1.0, 0.0, 1.0),
            snow: Color::new(1.0, 1.0, 1.0, 1.0),
        }
    }
}

pub struct BaseColoring<'a> {
    params: &'a BaseColoringParameters,
    game_state: &'a GameState,
}

impl<'a> BaseColoring<'a> {
    fn new(params: &'a BaseColoringParameters, game_state: &'a GameState) -> BaseColoring<'a> {
        BaseColoring { params, game_state }
    }

    fn get_groundwater(world: &World, position: &V2<usize>) -> f32 {
        world.tile_avg_groundwater(position).unwrap()
    }

    fn get_color(&self, world: &World, position: &V2<usize>) -> Color {
        let beach_level = self.game_state.params.world_gen.beach_level;
        let cliff_gradient = self.game_state.params.world_gen.cliff_gradient;
        let snow_temperature = self.game_state.params.snow_temperature;
        let max_gradient = world.get_max_abs_rise(&position);
        let min_elevation = world.get_lowest_corner(&position);
        if max_gradient > cliff_gradient {
            self.params.cliff
        } else if world
            .tile_avg_temperature(position)
            .map(|temperature| temperature < snow_temperature)
            .unwrap_or_default()
        {
            self.params.snow
        } else if min_elevation <= beach_level {
            self.params.beach
        } else {
            let groundwater = Self::get_groundwater(&world, &position);
            self.params
                .vegetation
                .blend(groundwater, &self.params.desert)
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
        if let WorldObject::Crop { .. } = self.game_state.world.get_cell_unsafe(tile).object {
            [None, None, None]
        } else {
            let color = Some(self.get_color(&self.game_state.world, tile));
            [color, color, color]
        }
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct TerritoryColoringParameters {
    territory_exclusive_alpha: f32,
    territory_non_exclusive_alpha: f32,
}

impl Default for TerritoryColoringParameters {
    fn default() -> TerritoryColoringParameters {
        TerritoryColoringParameters {
            territory_exclusive_alpha: 0.3,
            territory_non_exclusive_alpha: 0.15,
        }
    }
}

pub struct TerritoryColoring<'a> {
    params: &'a TerritoryColoringParameters,
    game_state: &'a GameState,
}

impl<'a> TerritoryColoring<'a> {
    fn new(
        params: &'a TerritoryColoringParameters,
        game_state: &'a GameState,
    ) -> TerritoryColoring<'a> {
        TerritoryColoring { params, game_state }
    }

    pub fn tile_color(&self, tile: &V2<usize>) -> Option<Color> {
        let game_state = self.game_state;
        if let Some(Claim {
            controller,
            duration,
            ..
        }) = game_state.territory.who_controls_tile(tile)
        {
            let settlement = game_state.settlements.get(&controller)?;
            let nation = self.nation(&settlement);
            let mut color = *nation.color();
            color.a = if *duration <= game_state.params.town_travel_duration {
                self.params.territory_exclusive_alpha
            } else {
                self.params.territory_non_exclusive_alpha
            };
            return Some(color);
        }
        None
    }

    fn nation(&self, settlement: &Settlement) -> &Nation {
        self.game_state
            .nations
            .get(&settlement.nation)
            .unwrap_or_else(|| panic!("Unknown nation {}", &settlement.nation))
    }
}

impl<'a> TerrainColoring<WorldCell> for TerritoryColoring<'a> {
    fn color(
        &self,
        _: &dyn Grid<WorldCell>,
        tile: &V2<usize>,
        _: &[V3<f32>; 3],
    ) -> [Option<Color>; 3] {
        let color = self.tile_color(tile);
        [color, color, color]
    }
}
