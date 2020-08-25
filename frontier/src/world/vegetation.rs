use serde::{Deserialize, Serialize};

pub const VEGETATION_TYPES: [VegetationType; 5] = [
    VegetationType::SnowTree,
    VegetationType::EvergreenTree,
    VegetationType::DeciduousTree,
    VegetationType::PalmTree,
    VegetationType::Cactus,
];

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub enum VegetationType {
    SnowTree,
    EvergreenTree,
    DeciduousTree,
    PalmTree,
    Cactus,
}

impl VegetationType {
    pub fn name(self) -> &'static str {
        match self {
            VegetationType::SnowTree => "snow_tree",
            VegetationType::EvergreenTree => "evergreen",
            VegetationType::DeciduousTree => "deciduous",
            VegetationType::PalmTree => "palm",
            VegetationType::Cactus => "cactus",
        }
    }

    pub fn height(self) -> f32 {
        match self {
            VegetationType::SnowTree => 0.01,
            VegetationType::EvergreenTree => 0.01,
            VegetationType::DeciduousTree => 0.01,
            VegetationType::PalmTree => 0.01,
            VegetationType::Cactus => 0.004,
        }
    }

    pub fn in_range_temperature(self, temperature: f32) -> bool {
        match self {
            VegetationType::SnowTree => temperature >= -5.0 && temperature < 0.0,
            VegetationType::EvergreenTree => temperature >= 0.0 && temperature < 10.0,
            VegetationType::DeciduousTree => temperature >= 10.0 && temperature < 20.0,
            VegetationType::PalmTree => temperature >= 20.0,
            VegetationType::Cactus => temperature >= 10.0,
        }
    }

    pub fn in_range_groundwater(self, groundwater: f32) -> bool {
        match self {
            VegetationType::SnowTree => groundwater >= 0.1,
            VegetationType::EvergreenTree => groundwater >= 0.1,
            VegetationType::DeciduousTree => groundwater >= 0.1,
            VegetationType::PalmTree => groundwater >= 0.1,
            VegetationType::Cactus => groundwater < 0.1,
        }
    }

    pub fn clumping(self) -> usize {
        match self {
            VegetationType::SnowTree => 1,
            VegetationType::EvergreenTree => 1,
            VegetationType::DeciduousTree => 1,
            VegetationType::PalmTree => 1,
            VegetationType::Cactus => 5,
        }
    }

    pub fn spread(self) -> f32 {
        match self {
            VegetationType::SnowTree => 0.5,
            VegetationType::EvergreenTree => 0.5,
            VegetationType::DeciduousTree => 0.33,
            VegetationType::PalmTree => 0.75,
            VegetationType::Cactus => 0.25,
        }
    }
}
