use serde::{Deserialize, Serialize};

use std::collections::HashMap;
use std::iter::once;
use std::time::Duration;

use commons::edge::Edge;

use crate::avatar::Vehicle;
use crate::travel_duration::EdgeDuration;

#[derive(Debug, Clone, Eq, Hash, PartialEq, Deserialize, Serialize)]
pub struct Bridge {
    pub edges: Vec<EdgeDuration>,
    pub vehicle: Vehicle,
    pub bridge_type: BridgeType,
}

#[derive(Debug, Clone, Eq, Hash, PartialEq, Deserialize, Serialize)]
pub enum BridgeType {
    Theoretical,
    Built,
}

impl Bridge {
    pub fn edge(&self) -> Edge {
        if let (Some(first), Some(last)) = (self.edges.first(), self.edges.last()) {
            match Edge::new_safe(first.from, last.to) {
                Ok(edge) => edge,
                _ => panic!("Bridge start and end positions must have same x or y coordinate."),
            }
        } else {
            panic!("Bridges must have at least one edge");
        }
    }

    pub fn duration(&self) -> Duration {
        self.edges.iter().flat_map(|edge| edge.duration).sum()
    }

    #[allow(clippy::needless_lifetimes)] // https://github.com/rust-lang/rust-clippy/issues/5787
    pub fn edge_durations<'a>(&'a self) -> impl Iterator<Item = EdgeDuration> + 'a {
        self.edges.iter().flat_map(|edge| {
            once(edge.clone()).chain(once(EdgeDuration {
                from: edge.to,
                to: edge.from,
                duration: edge.duration,
            }))
        })
    }
}

pub type Bridges = HashMap<Edge, Bridge>;
