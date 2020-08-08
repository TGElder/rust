use super::*;

use crate::route::RouteKey;
use commons::index2d::Vec2D;
use std::collections::{HashMap, HashSet};
use std::default::Default;

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct State {
    pub instructions: Vec<Instruction>,
    pub traffic: Traffic,
    pub edge_traffic: EdgeTraffic,
    pub route_to_ports: HashMap<RouteKey, HashSet<V2<usize>>>,
    pub build_queue: Vec<BuildInstruction>,
}

impl Default for State {
    fn default() -> State {
        State {
            instructions: vec![],
            traffic: Vec2D::new(1, 1, HashSet::new()),
            edge_traffic: hashmap! {},
            route_to_ports: hashmap! {},
            build_queue: vec![],
        }
    }
}
