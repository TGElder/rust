use crate::avatar::*;
use crate::road_builder::*;
use crate::territory::*;
use crate::world::*;
use crate::world_gen::*;
use commons::*;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::time::Duration;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct GameParams {
    pub world_gen: WorldGenParameters,
    pub avatar_travel: AvatarTravelParams,
    pub auto_road_travel: AutoRoadTravelParams,
    pub starting_distance_from_shore: usize,
    pub light_direction: V3<f32>,
    pub vegetation_exageration: f32,
    pub snow_temperature: f32,
    pub territory_duration: Duration,
    pub avatars: usize,
}

impl Default for GameParams {
    fn default() -> GameParams {
        GameParams {
            world_gen: WorldGenParameters::default(),
            avatar_travel: AvatarTravelParams::default(),
            auto_road_travel: AutoRoadTravelParams::default(),
            starting_distance_from_shore: 32,
            light_direction: v3(-1.0, 0.0, 1.0),
            vegetation_exageration: 100.0,
            snow_temperature: 0.0,
            territory_duration: Duration::from_secs(10),
            avatars: 4096,
        }
    }
}

#[derive(PartialEq, Debug, Serialize, Deserialize)]
pub struct GameState {
    pub world: World,
    pub game_micros: u128,
    pub params: GameParams,
    pub avatar_state: HashMap<String, AvatarState>,
    pub selected_avatar: Option<String>,
    pub follow_avatar: bool,
    pub territory: Territory,
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

    pub fn selected_avatar_name_and_state(&self) -> Option<(&str, &AvatarState)> {
        match &self.selected_avatar {
            Some(name) => match self.avatar_state.get(name) {
                Some(state) => Some((&name, state)),
                None => None,
            },
            None => None,
        }
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
        let mut avatar_state = HashMap::new();
        avatar_state.insert(
            "avatar".to_string(),
            AvatarState::Stationary {
                position: v2(1, 1),
                rotation: Rotation::Down,
                thinking: false,
            },
        );
        let game_state = GameState {
            territory: Territory::new(&world),
            world,
            game_micros: 123,
            params: GameParams::default(),
            avatar_state,
            selected_avatar: Some("avatar".to_string()),
            follow_avatar: false,
        };
        game_state.to_file("test_save");
        let loaded = GameState::from_file("test_save");
        assert_eq!(game_state, loaded);
    }
}
