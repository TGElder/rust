use super::*;

use serde::{Deserialize, Serialize};

use commons::grid::Grid;
use isometric::drawing::{draw_house, DrawHouseParams};
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
    pub fn new(params: HouseArtistParameters) -> HouseArtist {
        HouseArtist { parameters: params }
    }

    pub fn draw(
        &self,
        world: &World,
        from: &V2<usize>,
        to: &V2<usize>,
        territory_colors: &M<Option<Color>>,
    ) -> Vec<Command> {
        let mut out = vec![];
        for x in from.x..to.x {
            for y in from.y..to.y {
                let tile = v2(x, y);
                if let Some(WorldCell {
                    object: WorldObject::House,
                    ..
                }) = world.get_cell(&tile)
                {
                    let base_color =
                        unwrap_or!(territory_colors.get_cell_unsafe(&(tile - from)), continue);
                    let params = DrawHouseParams {
                        width: self.parameters.house_width,
                        height: self.parameters.house_height,
                        roof_height: self.parameters.house_roof_height,
                        base_color: *base_color,
                        light_direction: self.parameters.light_direction,
                    };
                    out.append(&mut draw_house(name(&tile), world, &tile, &params));
                }
            }
        }
        out
    }
}

fn name(at: &V2<usize>) -> String {
    format!("{:?}-house", at)
}
