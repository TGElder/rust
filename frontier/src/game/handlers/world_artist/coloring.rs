use crate::game::*;
use crate::world::*;
use commons::*;
use isometric::drawing::*;
use isometric::*;

fn sea_color() -> Color {
    Color::new(0.0, 0.0, 1.0, 1.0)
}

#[allow(clippy::type_complexity)]
pub fn create_coloring(
    game_state: &GameState,
) -> SeaLevelColoring<
    WorldCell,
    LayerColoring<
        WorldCell,
        LayerColoring<
            WorldCell,
            ShadedTileTerrainColoring<WorldCell, BaseColoring>,
            TerritoryColoring,
        >,
        FarmCandidateColoring,
    >,
> {
    let base = ShadedTileTerrainColoring::new(
        BaseColoring::new(game_state),
        game_state.params.light_direction,
    );
    let territory = TerritoryColoring::new(game_state);
    let farm_candidates = FarmCandidateColoring::new(game_state);
    let layers = LayerColoring::new(LayerColoring::new(base, territory), farm_candidates);
    SeaLevelColoring::new(
        layers,
        Some(sea_color()),
        game_state.params.world_gen.sea_level as f32,
    )
}

pub struct BaseColoring<'a> {
    game_state: &'a GameState,
}

impl<'a> BaseColoring<'a> {
    fn new(game_state: &'a GameState) -> BaseColoring {
        BaseColoring { game_state }
    }

    fn cliff_color() -> Color {
        Color::new(0.5, 0.4, 0.3, 1.0)
    }

    fn beach_color() -> Color {
        Color::new(1.0, 1.0, 0.0, 1.0)
    }

    fn desert_color() -> Color {
        Color::new(1.0, 0.8, 0.6, 1.0)
    }

    fn vegetation_color() -> Color {
        Color::new(0.0, 1.0, 0.0, 1.0)
    }

    fn snow_color() -> Color {
        Color::new(1.0, 1.0, 1.0, 1.0)
    }

    fn get_groundwater(world: &World, position: &V2<usize>) -> f32 {
        world
            .tile_average(&position, &|cell| {
                if !world.is_sea(&cell.position) {
                    Some(cell.climate.groundwater())
                } else {
                    None
                }
            })
            .unwrap()
    }

    fn get_color(&self, world: &World, position: &V2<usize>) -> Color {
        let beach_level = self.game_state.params.world_gen.beach_level;
        let cliff_gradient = self.game_state.params.world_gen.cliff_gradient;
        let snow_temperature = self.game_state.params.snow_temperature;
        let max_gradient = world.get_max_abs_rise(&position);
        let min_elevation = world.get_lowest_corner(&position);
        let cell = world.get_cell_unsafe(position);
        if max_gradient > cliff_gradient {
            Self::cliff_color()
        } else if cell.climate.temperature < snow_temperature {
            Self::snow_color()
        } else if min_elevation <= beach_level {
            Self::beach_color()
        } else {
            let groundwater = Self::get_groundwater(&world, &position);
            Self::vegetation_color().blend(groundwater, &Self::desert_color())
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
        if let WorldObject::Farm = self.game_state.world.get_cell_unsafe(tile).object {
            [None, None, None]
        } else {
            let color = Some(self.get_color(&self.game_state.world, tile));
            [color, color, color]
        }
    }
}

pub struct TerritoryColoring<'a> {
    game_state: &'a GameState,
}

impl<'a> TerritoryColoring<'a> {
    fn new(game_state: &'a GameState) -> TerritoryColoring {
        TerritoryColoring { game_state }
    }
}

impl<'a> TerrainColoring<WorldCell> for TerritoryColoring<'a> {
    fn color(
        &self,
        _: &dyn Grid<WorldCell>,
        tile: &V2<usize>,
        _: &[V3<f32>; 3],
    ) -> [Option<Color>; 3] {
        let mut color = self.game_state.tile_color(tile);
        if let Some(color) = &mut color {
            color.a = self.game_state.params.artist.territory_alpha;
        }
        [color, color, color]
    }
}

pub struct FarmCandidateColoring<'a> {
    game_state: &'a GameState,
}

impl<'a> FarmCandidateColoring<'a> {
    fn new(game_state: &'a GameState) -> FarmCandidateColoring {
        FarmCandidateColoring { game_state }
    }
}

impl<'a> TerrainColoring<WorldCell> for FarmCandidateColoring<'a> {
    fn color(
        &self,
        _: &dyn Grid<WorldCell>,
        tile: &V2<usize>,
        _: &[V3<f32>; 3],
    ) -> [Option<Color>; 3] {
        let highlight = self.game_state.params.artist.farm_candidate_highlight;
        let color = if self.game_state.is_farm_candidate(&tile) {
            Some(highlight)
        } else {
            None
        };
        [color, color, color]
    }
}
