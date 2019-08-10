use serde::{Deserialize, Serialize};
use std::default::*;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Vegetation {
    None,
    PalmTree,
    DeciduousTree,
    EvergreenTree,
    Cactus,
}

impl Vegetation {
    pub fn height(self) -> f32 {
        match self {
            Vegetation::None => 0.0,
            Vegetation::PalmTree => 1.0,
            Vegetation::DeciduousTree => 1.0,
            Vegetation::EvergreenTree => 1.0,
            Vegetation::Cactus => 0.5,
        }
    }

    pub fn in_range_temperature(self, temperature: f32) -> bool {
        match self {
            Vegetation::None => false,
            Vegetation::PalmTree => temperature >= 20.0,
            Vegetation::DeciduousTree => temperature >= 10.0 && temperature <= 20.0,
            Vegetation::EvergreenTree => temperature >= 0.0,
            Vegetation::Cactus => temperature >= 10.0,
        }
    }

    pub fn in_range_groundwater(self, groundwater: f32) -> bool {
        match self {
            Vegetation::None => false,
            Vegetation::PalmTree => groundwater >= 0.1,
            Vegetation::DeciduousTree => groundwater >= 0.1,
            Vegetation::EvergreenTree => groundwater >= 0.1,
            Vegetation::Cactus => groundwater <= 0.1,
        }
    }
}

impl Default for Vegetation {
    fn default() -> Vegetation {
        Vegetation::None
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Climate {
    pub temperature: f32,
    pub rainfall: f32,
    pub vegetation: Vegetation,
    pub vegetation_elevation: f32,
    pub river_water: f32,
}

impl Climate {
    pub fn groundwater(&self) -> f32 {
        self.rainfall.max(self.river_water)
    }
}
