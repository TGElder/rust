use super::vegetation::VegetationType;
use commons::V2;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub enum WorldObject {
    None,
    Vegetation {
        vegetation_type: VegetationType,
        offset: V2<f32>,
    },
    Crop {
        rotated: bool,
    },
    House,
    Pasture,
}
