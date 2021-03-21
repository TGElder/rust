use crate::actors::{BaseColors, TownArtistParameters};
use crate::args::Args;
use crate::avatar::AvatarTravelParams;
use crate::commons::persistence::Load;
use crate::homeland_start::HomelandEdge;
use crate::nation::{nation_descriptions, NationDescription};
use crate::road_builder::AutoRoadTravelParams;
use crate::simulation::SimulationParameters;
use crate::world_gen::WorldGenParameters;
use commons::{v3, V3};
use isometric::Color;

use serde::{Deserialize, Serialize};
use std::default::Default;
use std::time::Duration;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Parameters {
    pub seed: u64,
    pub power: usize,
    pub width: usize,
    pub reveal_all: bool,
    pub world_gen: WorldGenParameters,
    pub base_colors: BaseColors,
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
    pub simulation: SimulationParameters,
}

impl Default for Parameters {
    fn default() -> Parameters {
        Parameters {
            seed: 0,
            power: 0,
            width: 0,
            reveal_all: false,
            world_gen: WorldGenParameters::default(),
            base_colors: BaseColors::default(),
            avatar_travel: AvatarTravelParams::default(),
            auto_road_travel: AutoRoadTravelParams::default(),
            light_direction: v3(0.0, 8.0, -1.0),
            snow_temperature: 0.0,
            town_travel_duration: Duration::from_secs(60 * 60 * 6),
            avatars: 8192,
            homeland: HomelandParams::default(),
            avatar_color: Color::new(0.5, 0.5, 0.5, 1.0),
            town_artist: TownArtistParameters::default(),
            homeland_distance: Duration::from_secs(0),
            log_duration_threshold: None,
            label_padding: 2.0,
            nations: nation_descriptions(),
            default_speed: 3600.0,
            simulation: SimulationParameters::default(),
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

impl From<&Args> for Parameters {
    fn from(args: &Args) -> Self {
        match args {
            Args::New {
                power,
                seed,
                reveal_all,
                threads,
            } => Parameters {
                seed: *seed,
                power: *power,
                width: 2usize.pow(*power as u32),
                reveal_all: *reveal_all,
                homeland_distance: Duration::from_secs((3600.0 * 2f32.powf(*power as f32)) as u64),
                simulation: SimulationParameters {
                    threads: *threads,
                    ..SimulationParameters::default()
                },
                ..Parameters::default()
            },
            Args::Load { path, threads } => {
                let mut out = Self::load(&format!("{}.parameters", &path));
                out.simulation.threads = *threads;
                out
            }
        }
    }
}
