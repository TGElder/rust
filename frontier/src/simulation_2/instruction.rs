use super::*;
use crate::route::Route;
use crate::settlement::Settlement;

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub enum Instruction {
    Step,
    SettlementRef(V2<usize>),
    Settlement(Settlement),
    Demand(Demand),
    Route(Route),
    NewRoute(Route),
}
