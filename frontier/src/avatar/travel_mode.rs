use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq)]
pub enum TravelMode {
    Walk,
    Road,
    PlannedRoad,
    Stream,
    River,
    Sea,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Deserialize, Serialize)]
pub enum TravelModeClass {
    Land,
    Water,
}

impl TravelMode {
    pub fn class(&self) -> TravelModeClass {
        match self {
            TravelMode::Walk => TravelModeClass::Land,
            TravelMode::Road => TravelModeClass::Land,
            TravelMode::PlannedRoad => TravelModeClass::Land,
            TravelMode::Stream => TravelModeClass::Land,
            TravelMode::River => TravelModeClass::Water,
            TravelMode::Sea => TravelModeClass::Water,
        }
    }
}
