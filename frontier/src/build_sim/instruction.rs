use super::*;
use commons::edge::Edge;
use std::collections::HashSet;

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub enum Instruction {
    RefreshPositions(HashSet<V2<usize>>),
    RefreshEdges(HashSet<Edge>),
}
