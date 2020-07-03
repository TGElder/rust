use super::*;
use crate::route::{Route, RouteKey, RouteSet, RouteSetKey};
use crate::settlement::Settlement;

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub enum Instruction {
    Step,
    SettlementRef(V2<usize>),
    Settlement(Settlement),
    Demand(Demand),
    RouteSet {
        key: RouteSetKey,
        route_set: RouteSet,
    },
    RouteChange(RouteChange),
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
