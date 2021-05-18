use serde::{Deserialize, Serialize};

use std::collections::HashMap;
use std::iter::once;
use std::time::Duration;

use commons::edge::Edge;

use crate::avatar::Vehicle;
use crate::travel_duration::EdgeDuration;

#[derive(Debug, Clone, Eq, Hash, PartialEq, Deserialize, Serialize)]
pub struct Bridge {
    pub edge: Edge,
    pub duration: Duration,
    pub vehicle: Vehicle,
}

impl Bridge {
    pub fn edge_durations(&self) -> impl Iterator<Item = EdgeDuration> {
        once(EdgeDuration {
            from: *self.edge.from(),
            to: *self.edge.to(),
            duration: Some(self.duration),
        })
        .chain(once(EdgeDuration {
            from: *self.edge.to(),
            to: *self.edge.from(),
            duration: Some(self.duration),
        }))
    }
}

pub type Bridges = HashMap<Edge, Bridge>;
