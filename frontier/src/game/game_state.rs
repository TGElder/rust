use super::*;

use crate::avatar::*;
use crate::territory::*;
use crate::world::*;

use isometric::cell_traits::WithVisibility;
use isometric::Color;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, BufWriter};

#[derive(PartialEq, Debug, Serialize, Deserialize)]
pub struct GameState {
    pub world: World,
    pub game_micros: u128,
    pub params: GameParams,
    pub avatars: HashMap<String, Avatar>,
    pub selected_avatar: Option<String>,
    pub follow_avatar: bool,
    pub territory: Territory,
    pub speed: f32,
}

impl GameState {
    pub fn from_file(file_name: &str) -> GameState {
        let file = BufReader::new(File::open(file_name).unwrap());
        bincode::deserialize_from(file).unwrap()
    }

    pub fn to_file(&self, file_name: &str) {
        let mut file = BufWriter::new(File::create(file_name).unwrap());
        bincode::serialize_into(&mut file, &self).unwrap();
    }

    pub fn selected_avatar(&self) -> Option<&Avatar> {
        match &self.selected_avatar {
            Some(name) => self.avatars.get(name),
            None => None,
        }
    }

    pub fn is_farm_candidate(&self, position: &V2<usize>) -> bool {
        let constraints = &self.params.farm_constraints;
        let beach_level = self.params.world_gen.beach_level;
        let world = &self.world;
        world
            .get_cell(position)
            .map(|cell| {
                cell.is_visible()
                    && cell.object == WorldObject::None
                    && cell.climate.groundwater() >= constraints.min_groundwater
                    && cell.climate.temperature >= constraints.min_temperature
                    && world.get_max_abs_rise(position) <= constraints.max_slope
                    && world.get_lowest_corner(position) > beach_level
            })
            .unwrap_or(false)
    }

    pub fn tile_color(&self, tile: &V2<usize>) -> Option<Color> {
        let controlled_by = self.territory.who_controls_tile(&self.world, tile);
        if let Some(controller) = controlled_by {
            if let WorldObject::House(color) = self.world.get_cell_unsafe(&controller).object {
                return Some(color);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use commons::*;

    #[test]
    fn save_load_round_trip() {
        let world = World::new(
            M::from_vec(3, 3, vec![1.0, 1.0, 1.0, 1.0, 2.0, 1.0, 1.0, 1.0, 1.0]),
            0.5,
        );
        let mut avatars = HashMap::new();
        let name = "avatar";
        avatars.insert(
            name.to_string(),
            Avatar {
                name: name.to_string(),
                state: AvatarState::Stationary {
                    position: v2(1, 1),
                    rotation: Rotation::Down,
                },
                farm: Some(v2(9, 9)),
            },
        );
        let game_state = GameState {
            territory: Territory::new(&world),
            world,
            game_micros: 123,
            params: GameParams::default(),
            avatars,
            selected_avatar: Some(name.to_string()),
            follow_avatar: false,
            speed: 1.0,
        };
        game_state.to_file("test_save");
        let loaded = GameState::from_file("test_save");
        assert_eq!(game_state, loaded);
    }
}
