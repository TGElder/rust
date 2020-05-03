use super::*;

use serde::{Deserialize, Serialize};
use std::default::Default;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct SimParams {
    pub start_year: u128,
    pub natural_road: NaturalRoadSimParams,
    pub natural_town: NaturalTownSimParams,
    pub population_change: PopulationChangeSimParams,
    pub town_population: TownPopulationSimParams,
}

impl Default for SimParams {
    fn default() -> SimParams {
        SimParams {
            start_year: 0,
            natural_road: NaturalRoadSimParams::default(),
            natural_town: NaturalTownSimParams::default(),
            population_change: PopulationChangeSimParams::default(),
            town_population: TownPopulationSimParams::default(),
        }
    }
}
