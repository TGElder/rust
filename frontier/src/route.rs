use crate::resource::Resource;
use commons::V2;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::{Display, Formatter, Result};
use std::time::Duration;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Route {
    pub path: Vec<V2<usize>>,
    pub start_micros: u128,
    pub duration: Duration,
    pub traffic: usize,
}

#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct RouteKey {
    pub settlement: V2<usize>,
    pub resource: Resource,
    pub destination: V2<usize>,
}

impl Display for RouteKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(
            f,
            "{},{}-{:?}-{},{}",
            self.settlement.x,
            self.settlement.y,
            self.resource.name(),
            self.destination.x,
            self.destination.y
        )
    }
}

pub type RouteSet = HashMap<RouteKey, Route>;

#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct RouteSetKey {
    pub settlement: V2<usize>,
    pub resource: Resource,
}

impl From<&RouteKey> for RouteSetKey {
    fn from(route_key: &RouteKey) -> RouteSetKey {
        RouteSetKey {
            settlement: route_key.settlement,
            resource: route_key.resource,
        }
    }
}
