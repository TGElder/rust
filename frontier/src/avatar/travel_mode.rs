#[derive(Clone, Debug, PartialEq)]
pub enum TravelMode {
    Walk,
    Road,
    Stream,
    River,
    Sea,
}

#[derive(Debug, Eq, Hash, PartialEq)]
pub enum TravelModeClass {
    Land,
    Water,
}

impl TravelMode {
    pub fn class(&self) -> TravelModeClass {
        match self {
            TravelMode::Walk => TravelModeClass::Land,
            TravelMode::Road => TravelModeClass::Land,
            TravelMode::Stream => TravelModeClass::Land,
            TravelMode::River => TravelModeClass::Water,
            TravelMode::Sea => TravelModeClass::Water,
        }
    }
}
