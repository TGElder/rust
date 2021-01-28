use super::*;

use crate::world::*;

use serde::{Deserialize, Serialize};
use std::default::Default;
use std::fs::File;
use std::io::{BufReader, BufWriter};

#[derive(PartialEq, Debug, Serialize, Deserialize)]
pub struct GameState {
    pub world: World,
    pub params: GameParams,
}

impl Default for GameState {
    fn default() -> GameState {
        let world = World::new(M::zeros(1, 1), 0.0);
        GameState {
            params: GameParams::default(),
            world,
        }
    }
}

#[derive(PartialEq, Clone, Copy, Debug, Serialize, Deserialize)]
pub struct FirstVisit {
    pub when: u128,
    pub who: Option<V2<usize>>,
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
    use commons::*;

    #[test]
    fn save_load_round_trip() {
        let world = World::new(
            M::from_vec(3, 3, vec![1.0, 1.0, 1.0, 1.0, 2.0, 1.0, 1.0, 1.0, 1.0]),
            0.5,
        );
        let game_state = GameState {
            world,
            params: GameParams::default(),
        };
        game_state.to_file("test_save");
        let loaded = GameState::from_file("test_save");
        assert_eq!(game_state, loaded);
    }
}
