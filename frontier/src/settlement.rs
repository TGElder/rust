use commons::V2;
use isometric::Color;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct Settlement {
    pub class: SettlementClass,
    pub position: V2<usize>,
    pub color: Color,
    pub current_population: f64,
    pub target_population: f64,
    pub gap_half_life: Option<Duration>,
}

#[derive(Clone, Copy, Debug, PartialEq, Deserialize, Serialize)]
pub enum SettlementClass {
    Town,
    Homeland,
}
