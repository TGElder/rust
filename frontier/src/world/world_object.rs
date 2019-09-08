use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub enum VegetationType {
    EvergreenTree,
    DeciduousTree,
    PalmTree,
    Cactus,
}

impl VegetationType {
    pub fn height(self) -> f32 {
        match self {
            VegetationType::PalmTree => 1.0,
            VegetationType::DeciduousTree => 1.0,
            VegetationType::EvergreenTree => 1.0,
            VegetationType::Cactus => 0.5,
        }
    }

    pub fn in_range_temperature(self, temperature: f32) -> bool {
        match self {
            VegetationType::PalmTree => temperature >= 20.0,
            VegetationType::DeciduousTree => temperature >= 10.0 && temperature <= 20.0,
            VegetationType::EvergreenTree => temperature >= 0.0,
            VegetationType::Cactus => temperature >= 10.0,
        }
    }

    pub fn in_range_groundwater(self, groundwater: f32) -> bool {
        match self {
            VegetationType::PalmTree => groundwater >= 0.1,
            VegetationType::DeciduousTree => groundwater >= 0.1,
            VegetationType::EvergreenTree => groundwater >= 0.1,
            VegetationType::Cactus => groundwater <= 0.1,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub enum WorldObject {
    None,
    Vegetation(VegetationType),
    House,
}
