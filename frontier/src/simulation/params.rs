use std::default::Default;

use super::*;

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]

pub struct SimulationParams {
    pub traffic_to_population: f64,
    pub nation_flip_traffic_pc: f64,
}

impl Default for SimulationParams {
    fn default() -> SimulationParams {
        SimulationParams {
            traffic_to_population: 0.5,
            nation_flip_traffic_pc: 0.67,
        }
    }
}
