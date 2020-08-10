use std::default::Default;

use super::*;

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]

pub struct SimulationParams {
    pub traffic_to_population: f64,
}

impl Default for SimulationParams {
    fn default() -> SimulationParams {
        SimulationParams {
            traffic_to_population: 0.5,
        }
    }
}
