use super::*;

use serde::{Deserialize, Serialize};

use commons::grid::Grid;
use isometric::drawing::{create_and_update_house_drawing, House};
use isometric::Color;

#[derive(Clone, Copy, Debug, PartialEq, Deserialize, Serialize)]
pub struct HouseArtistParameters {
    pub house_width: f32,
    pub house_height: f32,
    pub house_roof_height: f32,
    pub light_direction: V3<f32>,
}

impl Default for HouseArtistParameters {
    fn default() -> HouseArtistParameters {
        HouseArtistParameters {
            house_width: 0.25,
            house_height: 0.25,
            house_roof_height: 0.5,
            light_direction: v3(0.0, 8.0, -1.0),
        }
    }
}

#[derive(Clone)]
pub struct HouseArtist {
    parameters: HouseArtistParameters,
}

impl HouseArtist {
    pub fn new(parameters: HouseArtistParameters) -> HouseArtist {
        HouseArtist { parameters }
    }

    pub fn draw(
        &self,
        world: &World,
        from: &V2<usize>,
        to: &V2<usize>,
        territory_colors: &M<Option<Color>>,
    ) -> Vec<Command> {
        let tiles = (from.x..to.x)
            .flat_map(|x| (from.y..to.y).map(move |y| v2(x, y)))
            .collect::<Vec<_>>();

        let mut houses = vec![];

        for tile in tiles.iter() {
            if !matches!(
                world.get_cell(&tile),
                Some(WorldCell {
                    object: WorldObject::House,
                    ..
                })
            ) {
                continue;
            }

            let base_color = unwrap_or!(territory_colors.get_cell_unsafe(&(tile - from)), continue);

            houses.push(House {
                position: tile,
                width: &self.parameters.house_width,
                height: &self.parameters.house_height,
                roof_height: &self.parameters.house_roof_height,
                base_color: &base_color,
                light_direction: &self.parameters.light_direction,
            });
        }

        if houses.is_empty() {
            return vec![];
        }

        let name = name(from);
        create_and_update_house_drawing(name, world, houses)
    }
}

fn name(at: &V2<usize>) -> String {
    format!("houses-{:?}", at)
}
