use std::default::Default;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Deserialize, Serialize)]

pub struct SimulationParameters {
    pub initial_town_population: f64,
    pub nation_flip_traffic_pc: f64,
    pub road_build_threshold: usize,
    pub town_removal_population: f64,
    pub traffic_to_population: f64,
}

impl Default for SimulationParameters {
    fn default() -> SimulationParameters {
        SimulationParameters {
            initial_town_population: 0.5,
            nation_flip_traffic_pc: 0.67,
            road_build_threshold: 8,
            town_removal_population: 0.25,
            traffic_to_population: 0.5,
        }
    }
}
