use super::*;

use crate::world::Resource;
use commons::v2;
use std::default::Default;

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct Demand {
    pub position: V2<usize>,
    pub resource: Resource,
    pub sources: usize,
    pub quantity: usize,
}

impl Default for Demand {
    fn default() -> Demand {
        Demand {
            position: v2(0, 0),
            resource: Resource::Crops,
            sources: 0,
            quantity: 0,
        }
    }
}
