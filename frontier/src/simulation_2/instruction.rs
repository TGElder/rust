use super::*;
use crate::route::{Route, RouteKey};
use crate::settlement::Settlement;
use std::collections::hash_set::HashSet;

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub enum Instruction {
    Step,
    SettlementRef(V2<usize>),
    Settlement(Settlement),
    Demand(Demand),
    Route {
        key: RouteKey,
        route: Route,
    },
    RouteChanged {
        key: RouteKey,
        positions_added: HashSet<V2<usize>>,
        positions_removed: HashSet<V2<usize>>,
    },
}
