use commons::V2;
use serde::{Deserialize, Serialize};

use std::collections::HashMap;
use std::time::Duration;

use commons::edge::Edge;

use crate::avatar::Vehicle;
use crate::travel_duration::EdgeDuration;

#[derive(Debug, Clone, Eq, Hash, PartialEq, Deserialize, Serialize)]
pub struct Bridge {
    edges: Vec<EdgeDuration>,
    vehicle: Vehicle,
    bridge_type: BridgeType,
}

#[derive(Debug, Clone, Eq, Hash, PartialEq, Deserialize, Serialize)]
pub enum BridgeType {
    Theoretical,
    Built,
}

impl Bridge {
    pub fn new(edges: Vec<EdgeDuration>, vehicle: Vehicle, bridge_type: BridgeType) -> Bridge {
        Self::validate_edges(&edges);
        Bridge {
            edges,
            vehicle,
            bridge_type,
        }
    }

    pub fn vehicle(&self) -> &Vehicle {
        &self.vehicle
    }

    pub fn bridge_type(&self) -> &BridgeType {
        &self.bridge_type
    }

    pub fn start(&self) -> V2<usize> {
        self.edges.first().unwrap().from
    }

    pub fn end(&self) -> V2<usize> {
        self.edges.last().unwrap().to
    }

    pub fn edge(&self) -> Edge {
        Edge::new(self.start(), self.end())
    }

    #[allow(clippy::needless_lifetimes)] // https://github.com/rust-lang/rust-clippy/issues/5787
    pub fn one_way_edges<'a>(
        &'a self,
        from: &V2<usize>,
    ) -> Box<dyn Iterator<Item = EdgeDuration> + 'a> {
        if self.start() == *from {
            Box::new(self.edges.iter().cloned())
        } else if self.end() == *from {
            Box::new(
                self.edges
                    .iter()
                    .map(|edge| EdgeDuration {
                        from: edge.to,
                        to: edge.from,
                        duration: edge.duration,
                    })
                    .rev(),
            )
        } else {
            panic!(
                "Position {} is at neither end of the bridge {:?}!",
                from, self.edges
            );
        }
    }

    #[allow(clippy::needless_lifetimes)] // https://github.com/rust-lang/rust-clippy/issues/5787
    pub fn both_way_edges<'a>(&'a self) -> impl Iterator<Item = EdgeDuration> + 'a {
        self.one_way_edges(&self.start())
            .chain(self.one_way_edges(&self.end()))
    }

    pub fn duration(&self) -> Duration {
        self.edges.iter().flat_map(|edge| edge.duration).sum()
    }

    fn validate_edges(edges: &[EdgeDuration]) {
        let first = edges
            .first()
            .unwrap_or_else(|| panic!("Bridge must have at least one edge"));
        let last = edges.last().unwrap();

        if Edge::new_safe(first.from, last.to).is_err() {
            panic!(
                "Bridge start {} and end {} must have same x or y coordinate!",
                first.from, last.to
            );
        }

        let next = edges.iter().skip(1);
        edges.iter().zip(next).for_each(|(a, b)| {
            if a.to != b.from {
                panic!("Bridge edges are not continuous: {:?}", edges)
            }
        });
    }
}

pub type Bridges = HashMap<Edge, Bridge>;
