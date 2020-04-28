use super::*;

use serde::{Deserialize, Serialize};
use std::default::Default;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct SimParams {
    pub start_year: u128,
    pub homeland_population: HomelandPopulationSimParams,
    pub natural_road: NaturalRoadSimParams,
    pub natural_town: NaturalTownSimParams,
    pub town_population: TownPopulationSimParams,
}

impl Default for SimParams {
    fn default() -> SimParams {
        SimParams {
            start_year: 0,
            homeland_population: HomelandPopulationSimParams::default(),
            natural_road: NaturalRoadSimParams::default(),
            natural_town: NaturalTownSimParams::default(),
            town_population: TownPopulationSimParams::default(),
        }
    }
}
