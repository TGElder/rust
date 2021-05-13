use serde::{Deserialize, Serialize};

use std::collections::HashSet;

use commons::edge::Edge;

#[derive(Eq, Hash, PartialEq, Deserialize, Serialize)]
pub struct Bridge {
    pub edge: Edge,
}

pub type Bridges = HashSet<Bridge>;
