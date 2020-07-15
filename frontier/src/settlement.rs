use commons::{v2, V2};
use serde::{Deserialize, Serialize};
use std::default::Default;
use std::time::Duration;

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct Settlement {
    pub class: SettlementClass,
    pub position: V2<usize>,
    pub name: String,
    pub nation: String,
    pub current_population: f64,
    pub target_population: f64,
    pub gap_half_life: Duration,
    pub last_population_update_micros: u128,
}

impl Default for Settlement {
    fn default() -> Settlement {
        Settlement {
            class: SettlementClass::default(),
            position: v2(0, 0),
            name: String::default(),
            nation: String::default(),
            current_population: 0.0,
            target_population: 0.0,
            gap_half_life: Duration::default(),
            last_population_update_micros: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Deserialize, Serialize)]
pub enum SettlementClass {
    Town,
    Homeland,
}

impl Default for SettlementClass {
    fn default() -> SettlementClass {
        SettlementClass::Town
    }
}
