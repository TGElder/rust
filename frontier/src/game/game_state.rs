use super::*;

use crate::avatar::*;
use crate::route::*;
use crate::settlement::*;
use crate::territory::*;
use crate::world::*;

use commons::index2d::Vec2D;

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
    pub settlements: HashMap<V2<usize>, Settlement>,
    pub selected_avatar: Option<String>,
    pub follow_avatar: bool,
    pub routes: HashMap<String, Route>,
    pub territory: Territory,
    pub first_visited: Vec2D<Option<u128>>,
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
}

#[cfg(test)]
mod tests {

    use super::*;
    use commons::*;
    use isometric::Color;

    #[test]
    fn save_load_round_trip() {
        let world = World::new(
            M::from_vec(3, 3, vec![1.0, 1.0, 1.0, 1.0, 2.0, 1.0, 1.0, 1.0, 1.0]),
            0.5,
        );
        let mut avatars = HashMap::new();
        avatars.insert(
            "avatar".to_string(),
            Avatar {
                name: "avatar".to_string(),
                state: AvatarState::Stationary {
                    position: v2(1, 1),
                    rotation: Rotation::Down,
                },
                load: AvatarLoad::Resource(Resource::Gold),
            },
        );
        let mut settlements = HashMap::new();
        settlements.insert(
            v2(3, 2),
            Settlement {
                class: SettlementClass::Town,
                position: v2(3, 2),
                color: Color::new(1.0, 0.0, 0.0, 1.0),
                name: "name".to_string(),
                current_population: 71.4,
                target_population: 41.1,
                gap_half_life: Some(Duration::from_secs(3)),
            },
        );
        let mut routes = HashMap::new();
        routes.insert(
            "route".to_string(),
            Route {
                resource: Resource::Bananas,
                settlement: v2(4, 1),
                path: vec![v2(1, 0), v2(2, 0)],
                traffic: 2,
                duration: Duration::from_secs(3),
            },
        );
        let game_state = GameState {
            territory: Territory::new(&world),
            first_visited: Vec2D::same_size_as(&world, None),
            world,
            game_micros: 123,
            params: GameParams::default(),
            avatars,
            settlements,
            selected_avatar: Some("avatar".to_string()),
            follow_avatar: false,
            routes,
            speed: 1.0,
        };
        game_state.to_file("test_save");
        let loaded = GameState::from_file("test_save");
        assert_eq!(game_state, loaded);
    }
}
