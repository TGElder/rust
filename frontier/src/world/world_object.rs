use super::vegetation::VegetationType;
use isometric::Color;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub enum WorldObject {
    None,
    Vegetation(VegetationType),
    House(Color),
    Farm { rotated: bool },
}
