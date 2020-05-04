use super::vegetation::VegetationType;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub enum WorldObject {
    None,
    Vegetation(VegetationType),
    Farm { rotated: bool },
}
