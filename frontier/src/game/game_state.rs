use super::*;

use crate::avatars::Avatars;
use crate::nation::Nation;
use crate::route::*;
use crate::settlement::*;
use crate::territory::*;
use crate::world::*;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::default::Default;
use std::fs::File;
use std::io::{BufReader, BufWriter};

#[derive(PartialEq, Debug, Serialize, Deserialize)]
pub struct GameState {
    pub world: World,
    pub game_micros: u128,
    pub params: GameParams,
    pub avatars: Avatars,
    pub nations: HashMap<String, Nation>,
    pub settlements: HashMap<V2<usize>, Settlement>,
    pub routes: HashMap<RouteSetKey, RouteSet>,
    pub territory: Territory,
    pub speed: f32,
}

impl Default for GameState {
    fn default() -> GameState {
        let world = World::new(M::zeros(1, 1), 0.0);
        GameState {
            game_micros: 0,
            params: GameParams::default(),
            avatars: Avatars::default(),
            nations: HashMap::new(),
            settlements: HashMap::new(),
            routes: HashMap::new(),
            territory: Territory::new(&world),
            speed: 0.0,
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
    use crate::avatar::{Avatar, Journey, Rotation, Vehicle};
    use crate::nation::{NationColors, NationDescription};
    use crate::resource::Resource;
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
                journey: Some(Journey::stationary(
                    &world,
                    v2(1, 1),
                    Vehicle::Boat,
                    Rotation::Down,
                )),
                color: Color::new(0.2, 0.4, 0.6, 0.8),
                skin_color: Color::new(0.3, 0.5, 0.7, 0.9),
            },
        );
        let mut nations = HashMap::new();
        nations.insert(
            "China".to_string(),
            Nation::from_description(&NationDescription {
                name: "China".to_string(),
                colors: NationColors {
                    primary: Color::new(1.0, 0.0, 0.0, 1.0),
                    skin: Color::new(0.0, 0.0, 1.0, 1.0),
                },
                town_name_file: "china".to_string(),
            }),
        );
        let mut settlements = HashMap::new();
        settlements.insert(
            v2(3, 2),
            Settlement {
                class: SettlementClass::Town,
                position: v2(3, 2),
                nation: "China".to_string(),
                name: "name".to_string(),
                current_population: 71.4,
                target_population: 41.1,
                gap_half_life: Duration::from_secs(3),
                last_population_update_micros: 81,
            },
        );
        let mut route_set = HashMap::new();
        route_set.insert(
            RouteKey {
                settlement: v2(4, 1),
                resource: Resource::Bananas,
                destination: v2(2, 0),
            },
            Route {
                path: vec![v2(1, 0), v2(2, 0)],
                traffic: 2,
                start_micros: 1232,
                duration: Duration::from_secs(3),
            },
        );
        let mut routes = HashMap::new();
        routes.insert(
            RouteSetKey {
                settlement: v2(4, 1),
                resource: Resource::Bananas,
            },
            route_set,
        );
        let game_state = GameState {
            territory: Territory::new(&world),
            world,
            game_micros: 123,
            params: GameParams::default(),
            avatars: Avatars {
                all: avatars,
                selected: Some("avatar".to_string()),
            },
            nations,
            settlements,
            routes,
            speed: 1.0,
        };
        game_state.to_file("test_save");
        let loaded = GameState::from_file("test_save");
        assert_eq!(game_state, loaded);
    }
}
