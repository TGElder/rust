use commons::V2;
use isometric::Color;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct Settlement {
    pub class: SettlementClass,
    pub position: V2<usize>,
    pub color: Color,
    pub population: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Deserialize, Serialize)]
pub enum SettlementClass {
    Town,
    Homeland,
}
