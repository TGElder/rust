use super::*;

use crate::route::RouteKey;
use std::collections::{HashMap, HashSet};
use std::default::Default;

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct State {
    pub params: SimulationParams,
    pub instructions: Vec<Instruction>,
    pub edge_traffic: EdgeTraffic,
    pub route_to_ports: HashMap<RouteKey, HashSet<V2<usize>>>,
}

impl Default for State {
    fn default() -> State {
        State {
            params: SimulationParams::default(),
            instructions: vec![],
            edge_traffic: hashmap! {},
            route_to_ports: hashmap! {},
        }
    }
}
