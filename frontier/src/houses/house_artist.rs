use crate::world::*;
use commons::*;
use isometric::coords::WorldCoord;
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
                basement_z: 0.0,
                base_color: Color::new(1.0, 0.0, 0.0, 1.0),
                light_direction,
            },
        }
    }

    fn get_name(position: &V2<usize>) -> String {
        format!("house-{:?}", position)
    }

    pub fn draw_house_at(&self, world: &World, position: &V2<usize>) -> Vec<Command> {
        let world_coord = world.snap_to_middle(WorldCoord::new(
            position.x as f32,
            position.y as f32,
            0 as f32,
        ));
        if let Some(world_coord) = world_coord {
            let basement_z = world.get_lowest_corner(position);
            return draw_house(
                Self::get_name(position),
                world_coord,
                &DrawHouseParams {
                    basement_z,
                    ..self.params
                },
            );
        }
        return vec![];
    }

    pub fn erase_house_at(&self, _: &World, position: &V2<usize>) -> Vec<Command> {
        vec![Command::Erase(Self::get_name(position))]
    }
}
