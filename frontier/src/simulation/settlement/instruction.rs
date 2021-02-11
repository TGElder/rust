use super::*;
use crate::route::{Route, RouteKey, RouteSet, RouteSetKey};
use std::time::Duration;

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub enum Instruction {
    UpdateSettlement(V2<usize>),
    Step,
    UpdateHomelandPopulation(V2<usize>),
    GetTerritory(V2<usize>),
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
pub struct Routes {
    pub key: RouteSetKey,
    pub route_set: RouteSet,
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
