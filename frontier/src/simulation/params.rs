use super::*;

use serde::{Deserialize, Serialize};
use std::default::Default;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct SimParams {
    pub start_year: u128,
    pub children: ChildrenParams,
    pub natural_road: NaturalRoadSimParams,
    pub natural_town: NaturalTownSimParams,
}

impl Default for SimParams {
    fn default() -> SimParams {
        SimParams {
            start_year: 0,
            children: ChildrenParams::default(),
            natural_road: NaturalRoadSimParams::default(),
            natural_town: NaturalTownSimParams::default(),
        }
    }
}
