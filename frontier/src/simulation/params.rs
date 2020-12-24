use std::default::Default;

use super::*;

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]

pub struct SimulationParams {
    pub road_build_threshold: usize,
    pub traffic_to_population: f64,
    pub nation_flip_traffic_pc: f64,
    pub initial_town_population: f64,
    pub town_removal_population: f64,
}

impl Default for SimulationParams {
    fn default() -> SimulationParams {
        SimulationParams {
            road_build_threshold: 8,
            traffic_to_population: 0.5,
            nation_flip_traffic_pc: 0.67,
            initial_town_population: 0.5,
            town_removal_population: 0.25,
        }
    }
}
