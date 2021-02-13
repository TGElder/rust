use crate::route::{Route, RouteKey, RouteSet, RouteSetKey};
use serde::{Deserialize, Serialize};
use std::time::Duration;

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
