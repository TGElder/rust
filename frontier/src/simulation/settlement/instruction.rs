use super::*;
use crate::route::{Route, RouteKey, RouteSet, RouteSetKey};
use crate::settlement::Settlement;
use std::time::Duration;

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub enum Instruction {
    UpdateSettlement(V2<usize>),
    Step,
    UpdateHomelandPopulation(V2<usize>),
    GetTerritory(V2<usize>),

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
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct TownTrafficSummary {
    pub nation: String,
    pub traffic_share: f64,
    pub total_duration: Duration,
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
