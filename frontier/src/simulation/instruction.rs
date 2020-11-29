use super::*;
use crate::route::{Route, RouteKey, RouteSet, RouteSetKey};
use crate::settlement::Settlement;
use commons::edge::Edge;
use std::collections::HashSet;

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub enum Instruction {
    Step,
    GetTerritory(V2<usize>),
    GetTownTraffic {
        settlement: Settlement,
        territory: HashSet<V2<usize>>,
    },
    UpdateTown {
        settlement: Settlement,
        traffic: Vec<TownTrafficSummary>,
    },
    UpdateCurrentPopulation(V2<usize>),
    GetDemand(Settlement),
    GetRoutes(Demand),
    GetRouteChanges {
        key: RouteSetKey,
        route_set: RouteSet,
    },
    ProcessRouteChanges(Vec<RouteChange>),
    RefreshPositions(HashSet<V2<usize>>),
    RefreshEdges(HashSet<Edge>),
    Build,
    VisibleLandPositions,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct TownTrafficSummary {
    pub nation: String,
    pub traffic_share: f64,
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
    NoChange {
        key: RouteKey,
        route: Route,
    },
}
