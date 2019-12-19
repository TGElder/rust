use crate::avatar::*;
use crate::road_builder::*;
use crate::world_gen::*;
use commons::*;
use isometric::Color;

use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct GameParams {
    pub world_gen: WorldGenParameters,
    pub artist: ArtistParams,
    pub avatar_travel: AvatarTravelParams,
    pub auto_road_travel: AutoRoadTravelParams,
    pub starting_distance_from_shore: usize,
    pub light_direction: V3<f32>,
    pub snow_temperature: f32,
    pub territory_duration: Duration,
    pub avatars: usize,
}

impl Default for GameParams {
    fn default() -> GameParams {
        GameParams {
            world_gen: WorldGenParameters::default(),
            artist: ArtistParams::default(),
            avatar_travel: AvatarTravelParams::default(),
            auto_road_travel: AutoRoadTravelParams::default(),
            starting_distance_from_shore: 32,
            light_direction: v3(-1.0, 0.0, 1.0),
            snow_temperature: 0.0,
            territory_duration: Duration::from_secs(10),
            avatars: 4096,
        }
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct ArtistParams {
    pub territory_highlight: Color,
}

impl Default for ArtistParams {
    fn default() -> ArtistParams {
        ArtistParams {
            territory_highlight: Color::new(0.0, 0.0, 1.0, 0.25),
        }
    }
}
