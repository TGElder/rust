use super::*;
use std::cmp::Ordering;

#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub struct BuildInstruction {
    pub what: Build,
    pub when: u128,
}

impl Ord for BuildInstruction {
    fn cmp(&self, other: &BuildInstruction) -> Ordering {
        self.when.cmp(&other.when).reverse()
    }
}

impl PartialOrd for BuildInstruction {
    fn partial_cmp(&self, other: &BuildInstruction) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for BuildInstruction {}
