use super::*;

use serde::{Deserialize, Serialize};
use std::default::Default;
use std::fs::File;
use std::io::{BufReader, BufWriter};

#[derive(PartialEq, Debug, Serialize, Deserialize)]
pub struct GameState {
    pub params: GameParams,
}

impl Default for GameState {
    fn default() -> GameState {
        GameState {
            params: GameParams::default(),
        }
    }
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
        let game_state = GameState {
            params: GameParams::default(),
        };
        game_state.to_file("test_save");
        let loaded = GameState::from_file("test_save");
        assert_eq!(game_state, loaded);
    }
}
