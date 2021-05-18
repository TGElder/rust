use serde::{Deserialize, Serialize};

use std::collections::HashMap;
use std::time::Duration;

use commons::edge::Edge;

use crate::avatar::Vehicle;

#[derive(Debug, Clone, Eq, Hash, PartialEq, Deserialize, Serialize)]
pub struct Bridge {
    pub edge: Edge,
    pub duration: Duration,
    pub vehicle: Vehicle,
}

pub type Bridges = HashMap<Edge, Bridge>;
