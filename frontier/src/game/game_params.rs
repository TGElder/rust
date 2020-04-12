use crate::avatar::*;
use crate::game_event_consumers::WorldColoringParameters;
use crate::road_builder::*;
use crate::simulation::*;
use crate::world_gen::*;
use commons::*;
use isometric::Color;

use serde::{Deserialize, Serialize};
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
    pub farm_constraints: FarmConstraints,
    pub snow_temperature: f32,
    pub town_exclusive_duration: Duration,
    pub town_travel_duration: Duration,
    pub avatars: usize,
    pub sim: SimParams,
    pub house_color: Color,
    pub log_duration_threshold: Option<Duration>,
}

impl GameParams {
    pub fn new(seed: u64) -> GameParams {
        GameParams {
            seed,
            world_gen: WorldGenParameters::default(),
            world_coloring: WorldColoringParameters::default(),
            avatar_travel: AvatarTravelParams::default(),
            auto_road_travel: AutoRoadTravelParams::default(),
            starting_distance_from_shore: 32,
            light_direction: v3(-1.0, 0.0, 1.0),
            farm_constraints: FarmConstraints::default(),
            snow_temperature: 0.0,
            town_exclusive_duration: Duration::from_secs(60 * 60 * 3),
            town_travel_duration: Duration::from_secs(60 * 60 * 12),
            avatars: 4096,
            sim: SimParams::default(),
            house_color: Color::new(1.0, 0.0, 0.0, 1.0),
            log_duration_threshold: None,
        }
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct FarmConstraints {
    pub min_groundwater: f32,
    pub max_slope: f32,
    pub min_temperature: f32,
}

impl Default for FarmConstraints {
    fn default() -> FarmConstraints {
        FarmConstraints {
            min_groundwater: 0.1,
            max_slope: 0.2,
            min_temperature: 0.0,
        }
    }
}
