use super::*;
use crate::route::Route;
use crate::settlement::Settlement;
use std::collections::hash_set::HashSet;

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub enum Instruction {
    Step,
    SettlementRef(V2<usize>),
    Settlement(Settlement),
    Demand(Demand),
    Route(Route),
    RouteChanged {
        new_route: Route,
        positions_added: HashSet<V2<usize>>,
        positions_removed: HashSet<V2<usize>>,
    },
}
