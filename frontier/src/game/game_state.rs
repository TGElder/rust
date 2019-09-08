use crate::avatar::*;
use crate::world::*;
use crate::world_gen::*;
use commons::*;

use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufReader, BufWriter};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct GameParams {
    pub world_gen: WorldGenParameters,
    pub avatar_travel: AvatarTravelParams,
    pub starting_distance_from_shore: usize,
    pub light_direction: V3<f32>,
}

impl Default for GameParams {
    fn default() -> GameParams {
        GameParams {
            world_gen: WorldGenParameters::default(),
            avatar_travel: AvatarTravelParams::default(),
            starting_distance_from_shore: 32,
            light_direction: v3(-1.0, 0.0, 1.0),
        }
    }
}

#[derive(PartialEq, Debug, Serialize, Deserialize)]
pub struct GameState {
    pub world: World,
    pub game_micros: u128,
    pub params: GameParams,
    pub avatar_state: AvatarState,
    pub follow_avatar: bool,
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
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn save_load_round_trip() {
        let world = World::new(
            M::from_vec(3, 3, vec![1.0, 1.0, 1.0, 1.0, 2.0, 1.0, 1.0, 1.0, 1.0]),
            0.5,
        );
        let game_state = GameState {
            world,
            game_micros: 123,
            params: GameParams::default(),
            avatar_state: AvatarState::Stationary {
                position: v2(1, 1),
                rotation: Rotation::Down,
            },
            follow_avatar: false,
        };
        game_state.to_file("test_save");
        let loaded = GameState::from_file("test_save");
        assert_eq!(game_state, loaded);
    }
}
