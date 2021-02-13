use crate::resource::Resource;
use commons::{v2, V2};
use serde::{Deserialize, Serialize};
use std::default::Default;

#[derive(Clone, Copy, Debug, PartialEq, Deserialize, Serialize)]
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
