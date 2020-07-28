use super::*;
use crate::route::{Route, RouteKey, RouteSet, RouteSetKey};
use crate::settlement::Settlement;
use commons::edge::Edge;
use std::collections::HashSet;
use std::time::Duration;

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub enum Instruction {
    Step,
    GetTerritory(V2<usize>),
    UpdateTown {
        settlement: Settlement,
        territory: HashSet<V2<usize>>,
    },
    UpdateCurrentPopulation(V2<usize>),
    GetDemand(Settlement),
    GetRoutes(Demand),
    GetRouteChanges {
        key: RouteSetKey,
        route_set: RouteSet,
    },
    Build,
    VisibleLandPositions(usize),
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
pub struct TrafficSummary {
    pub position: V2<usize>,
    pub controller: Option<V2<usize>>,
    pub routes: Vec<RouteSummary>,
    pub adjacent: Vec<Tile>,
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

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]

pub struct EdgeTrafficSummary {
    pub edge: Edge,
    pub road_status: RoadStatus,
    pub routes: Vec<EdgeRouteSummary>,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub enum RoadStatus {
    Built,
    Planned(u128),
    Suitable,
    Unsuitable,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct EdgeRouteSummary {
    pub traffic: usize,
    pub first_visit: u128,
}
