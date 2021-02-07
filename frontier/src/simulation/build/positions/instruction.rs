use super::*;
use std::collections::HashSet;

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub enum Instruction {
    RefreshPositions(HashSet<V2<usize>>),
}
