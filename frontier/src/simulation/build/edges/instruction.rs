use super::*;
use commons::edge::Edge;
use std::collections::HashSet;

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub enum Instruction {
    RefreshEdges(HashSet<Edge>),
}
