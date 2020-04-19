use crate::avatar::*;
use crate::game_event_consumers::WorldColoringParameters;
use crate::road_builder::*;
use crate::simulation::*;
use crate::world_gen::*;
use commons::*;
use isometric::Color;

use serde::{Deserialize, Serialize};
use std::default::Default;
use std::time::Duration;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct GameParams {
    pub seed: u64,
    pub world_gen: WorldGenParameters,
    pub world_coloring: WorldColoringParameters,
    pub avatar_travel: AvatarTravelParams,
    pub auto_road_travel: AutoRoadTravelParams,
    pub starting_distance_from_shore: usize,
    pub light_direction: V3<f32>,
    pub snow_temperature: f32,
    pub town_exclusive_duration: Duration,
    pub town_travel_duration: Duration,
    pub avatars: usize,
    pub sim: SimParams,
    pub house_color: Color,
    pub log_duration_threshold: Option<Duration>,
    pub old_world_population: usize,
}

impl Default for GameParams {
    fn default() -> GameParams {
        GameParams {
            seed: 0,
            world_gen: WorldGenParameters::default(),
            world_coloring: WorldColoringParameters::default(),
            avatar_travel: AvatarTravelParams::default(),
            auto_road_travel: AutoRoadTravelParams::default(),
            starting_distance_from_shore: 32,
            light_direction: v3(-1.0, 0.0, 1.0),
            snow_temperature: 0.0,
            town_exclusive_duration: Duration::from_secs(60 * 60 * 6),
            town_travel_duration: Duration::from_secs(60 * 60 * 6),
            avatars: 4096,
            sim: SimParams::default(),
            house_color: Color::new(1.0, 0.0, 0.0, 1.0),
            log_duration_threshold: None,
            old_world_population: 64,
        }
    }
}
