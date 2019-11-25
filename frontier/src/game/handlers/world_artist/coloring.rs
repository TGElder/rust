use crate::game::*;
use crate::world::*;
use commons::*;
use isometric::drawing::*;
use isometric::*;

fn sea_color() -> Color {
    Color::new(0.0, 0.0, 1.0, 1.0)
}

pub fn create_coloring(
    game_state: &GameState,
) -> SeaLevelColoring<
    WorldCell,
    LayerColoring<WorldCell, ShadedTileTerrainColoring<WorldCell, BaseColoring>, TerritoryColoring>,
> {
    let base = ShadedTileTerrainColoring::new(
        BaseColoring::new(game_state),
        game_state.params.light_direction,
    );
    let territory = TerritoryColoring::new(game_state);
    let layers = LayerColoring::new(base, territory);
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
        let color = Some(self.get_color(&self.game_state.world, tile));
        [color, color, color]
    }
}

pub struct TerritoryColoring<'a> {
    game_state: &'a GameState,
    color: Color,
}

impl<'a> TerritoryColoring<'a> {
    fn new(game_state: &'a GameState) -> TerritoryColoring {
        TerritoryColoring {
            game_state,
            color: Color::new(1.0, 0.0, 0.0, 0.25),
        }
    }
}

impl<'a> TerrainColoring<WorldCell> for TerritoryColoring<'a> {
    fn color(
        &self,
        _: &dyn Grid<WorldCell>,
        tile: &V2<usize>,
        _: &[V3<f32>; 3],
    ) -> [Option<Color>; 3] {
        let world = &self.game_state.world;
        let controlled = world
            .get_corners(tile)
            .iter()
            .filter(|corner| world.in_bounds(corner))
            .all(|corner| !self.game_state.territory.who_controls(corner).is_empty());
        let color = if controlled { Some(self.color) } else { None };
        [color, color, color]
    }
}
