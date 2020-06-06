use super::*;

use serde::{Deserialize, Serialize};
use std::default::Default;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct SimParams {
    pub start_year: u128,
    pub natural_road: NaturalRoadSimParams,
    pub town_population: TownPopulationSimParams,
}

impl Default for SimParams {
    fn default() -> SimParams {
        SimParams {
            start_year: 0,
            natural_road: NaturalRoadSimParams::default(),
            town_population: TownPopulationSimParams::default(),
        }
    }
}
