use crate::world::*;
use commons::*;
use isometric::drawing::{draw_house, DrawHouseParams};
use isometric::Color;
use isometric::Command;

pub struct HouseArtist {
    params: DrawHouseParams,
}

impl HouseArtist {
    pub fn new(light_direction: V3<f32>) -> HouseArtist {
        HouseArtist {
            params: DrawHouseParams {
                width: 0.25,
                height: 0.5,
                roof_height: 0.5,
                base_color: Color::new(1.0, 0.0, 0.0, 1.0),
                light_direction,
            },
        }
    }

    fn get_name(position: &V2<usize>) -> String {
        format!("house-{:?}", position)
    }

    pub fn draw_house_at(
        &self,
        world: &World,
        position: &V2<usize>,
        base_color: Color,
        height: f32,
        roof_height: f32,
    ) -> Vec<Command> {
        draw_house(
            Self::get_name(position),
            world,
            position,
            &DrawHouseParams {
                base_color,
                height,
                roof_height,
                ..self.params
            },
        )
    }

    pub fn erase_house_at(&self, _: &World, position: &V2<usize>) -> Vec<Command> {
        vec![Command::Erase(Self::get_name(position))]
    }
}
