use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq)]
pub enum AvatarTravelMode {
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

impl AvatarTravelMode {
    pub fn class(&self) -> TravelModeClass {
        match self {
            AvatarTravelMode::Walk => TravelModeClass::Land,
            AvatarTravelMode::Road => TravelModeClass::Land,
            AvatarTravelMode::PlannedRoad => TravelModeClass::Land,
            AvatarTravelMode::Stream => TravelModeClass::Land,
            AvatarTravelMode::River => TravelModeClass::Water,
            AvatarTravelMode::Sea => TravelModeClass::Water,
        }
    }
}
