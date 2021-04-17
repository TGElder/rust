use super::*;

use commons::grid::Grid;
use isometric::drawing::{draw_house, DrawHouseParams};
use isometric::Color;

#[derive(Clone)]
pub struct HouseArtist {}

impl HouseArtist {
    pub fn new() -> HouseArtist {
        HouseArtist {}
    }

    pub fn draw(
        &self,
        world: &World,
        from: &V2<usize>,
        to: &V2<usize>,
        coloring: &WorldColoring,
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
                    let params = DrawHouseParams {
                        width: 0.25,
                        height: 0.25,
                        roof_height: 0.5,
                        base_color: coloring
                            .overlay
                            .color(world, &tile, &[v3(0.0, 0.0, 0.0); 3])[0]
                            .unwrap_or_else(|| Color::new(1.0, 1.0, 1.0, 1.0)),
                        light_direction: v3(0.0, 8.0, -1.0),
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
