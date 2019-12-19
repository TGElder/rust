use crate::world::*;
use commons::*;
use isometric::drawing::textured_tile;
use isometric::Color;
use isometric::Command;

const TEXTURE: &str = "farm.png";

pub struct FarmArtist {}

impl FarmArtist {
    pub fn new() -> FarmArtist {
        FarmArtist {}
    }

    fn get_name(position: &V2<usize>) -> String {
        format!("farm-{:?}", position)
    }

    pub fn draw_farm_at(
        &self,
        world: &World,
        sea_level: f32,
        position: &V2<usize>,
        color: &Color,
    ) -> Vec<Command> {
        textured_tile(
            Self::get_name(position),
            world,
            sea_level,
            position,
            color,
            TEXTURE.to_string(),
        )
    }

    pub fn erase_farm_at(&self, _: &World, position: &V2<usize>) -> Vec<Command> {
        vec![Command::Erase(Self::get_name(position))]
    }
}
