use super::*;
use crate::route::{Route, RouteKey, RouteSet, RouteSetKey};
use crate::settlement::Settlement;
use std::collections::HashSet;
use std::time::Duration;

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub enum Instruction {
    Step,
    SettlementRef(V2<usize>),
    Territory {
        settlement: Settlement,
        territory: HashSet<V2<usize>>,
    },
    Settlement(Settlement),
    Demand(Demand),
    RouteSet {
        key: RouteSetKey,
        route_set: RouteSet,
    },
    RouteChange(RouteChange),
    TrafficChange(V2<usize>),
    Traffic {
        position: V2<usize>,
        controller: Option<V2<usize>>,
        routes: Vec<RouteSummary>,
        adjacent: Vec<Tile>,
    },
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub enum RouteChange {
    New {
        key: RouteKey,
        route: Route,
    },
    Updated {
        key: RouteKey,
        old: Route,
        new: Route,
    },
    Removed {
        key: RouteKey,
        route: Route,
    },
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct Tile {
    pub position: V2<usize>,
    pub settlement: Option<Settlement>,
    pub sea: bool,
    pub visible: bool,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct RouteSummary {
    pub traffic: usize,
    pub origin: V2<usize>,
    pub destination: V2<usize>,
    pub nation: String,
    pub first_visit: u128,
    pub duration: Duration,
}
