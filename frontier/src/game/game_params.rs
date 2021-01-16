use crate::actors::{TownArtistParameters, WorldColoringParameters};
use crate::avatar::*;
use crate::homeland_start::HomelandEdge;
use crate::nation::{nation_descriptions, NationDescription};
use crate::road_builder::*;
use crate::world_gen::*;
use commons::*;
use isometric::Color;

use serde::{Deserialize, Serialize};
use std::default::Default;
use std::time::Duration;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct GameParams {
    pub seed: u64,
    pub reveal_all: bool,
    pub world_gen: WorldGenParameters,
    pub world_coloring: WorldColoringParameters,
    pub avatar_travel: AvatarTravelParams,
    pub auto_road_travel: AutoRoadTravelParams,
    pub light_direction: V3<f32>,
    pub snow_temperature: f32,
    pub town_travel_duration: Duration,
    pub avatars: usize,
    pub homeland: HomelandParams,
    pub avatar_color: Color,
    pub town_artist: TownArtistParameters,
    pub homeland_distance: Duration,
    pub log_duration_threshold: Option<Duration>,
    pub label_padding: f32,
    pub nations: Vec<NationDescription>,
    pub default_speed: f32,
}

impl Default for GameParams {
    fn default() -> GameParams {
        GameParams {
            seed: 0,
            reveal_all: false,
            world_gen: WorldGenParameters::default(),
            world_coloring: WorldColoringParameters::default(),
            avatar_travel: AvatarTravelParams::default(),
            auto_road_travel: AutoRoadTravelParams::default(),
            light_direction: v3(0.0, 8.0, -1.0),
            snow_temperature: 0.0,
            town_travel_duration: Duration::from_secs(60 * 60 * 6),
            avatars: 1024,
            homeland: HomelandParams::default(),
            avatar_color: Color::new(0.5, 0.5, 0.5, 1.0),
            town_artist: TownArtistParameters::default(),
            homeland_distance: Duration::from_secs(0),
            log_duration_threshold: None,
            label_padding: 2.0,
            nations: nation_descriptions(),
            default_speed: 3600.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HomelandParams {
    pub count: usize,
    pub edges: Vec<HomelandEdge>,
}

impl Default for HomelandParams {
    fn default() -> HomelandParams {
        HomelandParams {
            count: 8,
            edges: vec![HomelandEdge::East, HomelandEdge::West],
        }
    }
}
