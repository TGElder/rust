use crate::world::Resource;
use commons::V2;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(PartialEq, Debug, Serialize, Deserialize)]
pub struct Route {
    pub resource: Resource,
    pub settlement: V2<usize>,
    pub path: Vec<V2<usize>>,
    pub duration: Duration,
    pub traffic: usize,
}
