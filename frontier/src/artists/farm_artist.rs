use super::*;

use isometric::drawing::{textured_tiles, TerrainColoring, TexturedTile};
use std::f32::consts::PI;

const TEXTURE: &str = "resources/textures/farm.png";

pub struct FarmArtist {}

impl FarmArtist {
    pub fn new() -> FarmArtist {
        FarmArtist {}
    }

    pub fn draw(
        &self,
        world: &World,
        coloring: &dyn TerrainColoring<WorldCell>,
        from: &V2<usize>,
        to: &V2<usize>,
    ) -> Vec<Command> {
        let mut tiles = vec![];
        for x in from.x..to.x {
            for y in from.y..to.y {
                let tile = v2(x, y);
                if let Some(WorldCell {
                    object: WorldObject::Farm { rotated },
                    ..
                }) = world.get_cell(&tile)
                {
                    let rotation = if *rotated { PI / 2.0 } else { 0.0 };
                    tiles.push(TexturedTile { tile, rotation });
                }
            }
        }
        textured_tiles(
            name(from),
            world,
            world.sea_level(),
            &tiles,
            coloring,
            TEXTURE.to_string(),
        )
    }
}

fn name(from: &V2<usize>) -> String {
    format!("{:?}-farms", from)
}
