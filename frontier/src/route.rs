use crate::world::Resource;
use commons::V2;
use serde::{Deserialize, Serialize};

#[derive(PartialEq, Debug, Serialize, Deserialize)]
pub struct Route {
    pub resource: Resource,
    pub path: Vec<V2<usize>>,
}
