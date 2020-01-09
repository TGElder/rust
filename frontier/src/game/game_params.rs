use crate::avatar::*;
use crate::road_builder::*;
use crate::world_gen::*;
use commons::*;
use isometric::Color;

use chrono::{NaiveDate, NaiveDateTime};
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
    pub farm_constraints: FarmConstraints,
    pub snow_temperature: f32,
    pub territory_duration: Duration,
    pub avatars: usize,
    pub history_start_date: NaiveDateTime,
    pub play_start_date: NaiveDateTime,
}

impl GameParams {
    pub fn start_micros(&self) -> u128 {
        if self.history_start_date > self.play_start_date {
            panic!(
                "History start date {:?} must be before play start date {:?}",
                self.history_start_date, self.play_start_date
            );
        }
        self.play_start_date
            .signed_duration_since(self.history_start_date)
            .num_microseconds()
            .unwrap() as u128
    }
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
            farm_constraints: FarmConstraints::default(),
            snow_temperature: 0.0,
            territory_duration: Duration::from_secs(60 * 60 * 4),
            avatars: 4096,
            history_start_date: NaiveDate::from_ymd(1400, 1, 1).and_hms(0, 0, 0),
            play_start_date: NaiveDate::from_ymd(1500, 1, 1).and_hms(0, 0, 0),
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

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct ArtistParams {
    pub territory_alpha: f32,
    pub farm_candidate_highlight: Color,
}

impl Default for ArtistParams {
    fn default() -> ArtistParams {
        ArtistParams {
            territory_alpha: 0.25,
            farm_candidate_highlight: Color::new(0.0, 1.0, 0.0, 0.0),
        }
    }
}
