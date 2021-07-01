use commons::V2;
use serde::{Deserialize, Serialize};

use std::collections::{HashMap, HashSet};
use std::iter::once;
use std::time::Duration;
use std::{error, fmt};

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

pub type Bridges = HashMap<Edge, HashSet<Bridge>>;

impl Bridge {
    pub fn new(
        edges: Vec<EdgeDuration>,
        vehicle: Vehicle,
        bridge_type: BridgeType,
    ) -> Result<Bridge, InvalidBridge> {
        Self::validate_edges(&edges)?;
        Ok(Bridge {
            edges,
            vehicle,
            bridge_type,
        })
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

    #[allow(clippy::needless_lifetimes)] // https://github.com/rust-lang/rust-clippy/issues/5787
    pub fn edges_one_way<'a>(
        &'a self,
        from: &V2<usize>,
    ) -> Box<dyn Iterator<Item = EdgeDuration> + 'a> {
        if self.start() == *from {
            Box::new(self.edges.iter().cloned())
        } else if self.end() == *from {
            Box::new(self.edges_reversed())
        } else {
            panic!(
                "Position {} is at neither end of the bridge {:?}!",
                from, self.edges
            );
        }
    }

    #[allow(clippy::needless_lifetimes)] // https://github.com/rust-lang/rust-clippy/issues/5787
    fn edges_reversed<'a>(&'a self) -> impl Iterator<Item = EdgeDuration> + 'a {
        self.edges
            .iter()
            .map(|edge| EdgeDuration {
                from: edge.to,
                to: edge.from,
                duration: edge.duration,
            })
            .rev()
    }

    pub fn total_edge(&self) -> Edge {
        Edge::new(self.start(), self.end())
    }

    pub fn total_duration(&self) -> Duration {
        self.edges.iter().flat_map(|edge| edge.duration).sum()
    }

    #[allow(clippy::needless_lifetimes)] // https://github.com/rust-lang/rust-clippy/issues/5787
    pub fn total_edge_durations<'a>(&'a self) -> impl Iterator<Item = EdgeDuration> + 'a {
        let edge = self.total_edge();
        let duration = self.total_duration();
        once(EdgeDuration {
            from: *edge.to(),
            to: *edge.from(),
            duration: Some(duration),
        })
        .chain(once(EdgeDuration {
            from: *edge.from(),
            to: *edge.to(),
            duration: Some(duration),
        }))
    }

    fn validate_edges(edges: &[EdgeDuration]) -> Result<(), InvalidBridge> {
        let first = unwrap_or!(edges.first(), return Err(InvalidBridge::Empty));
        let last = edges.last().unwrap();

        if Edge::new_safe(first.from, last.to).is_err() {
            return Err(InvalidBridge::Diagonal);
        }

        let next = edges.iter().skip(1);
        edges.iter().zip(next).try_for_each(|(a, b)| {
            if a.to != b.from {
                Err(InvalidBridge::Discontinuous)
            } else {
                Ok(())
            }
        })
    }
}

pub trait BridgesExt {
    fn get_lowest_duration_bridge(&self, edge: &Edge) -> Option<&Bridge>;
    fn count_bridges_at(&self, position: &V2<usize>, bridge_type: &BridgeType) -> usize;
}

impl BridgesExt for Bridges {
    fn get_lowest_duration_bridge(&self, edge: &Edge) -> Option<&Bridge> {
        self.get(edge)
            .and_then(|bridges| bridges.iter().min_by_key(|bridge| bridge.total_duration()))
    }

    fn count_bridges_at(&self, position: &V2<usize>, bridge_type: &BridgeType) -> usize {
        self.iter()
            .filter(|(key, _)| key.from() == position || key.to() == position)
            .filter(|(_, bridges)| {
                bridges
                    .iter()
                    .any(|bridge| bridge.bridge_type() == bridge_type)
            })
            .count()
    }
}

#[derive(Debug, PartialEq)]
pub enum InvalidBridge {
    Empty,
    Diagonal,
    Discontinuous,
}

impl fmt::Display for InvalidBridge {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InvalidBridge::Empty => write!(f, "Bridge must have at least one edge"),
            InvalidBridge::Diagonal => {
                write!(f, "Bridge start and end must have same x or y coordinate")
            }
            InvalidBridge::Discontinuous => write!(f, "Bridge edges are not continuous"),
        }
    }
}

impl error::Error for InvalidBridge {}

#[cfg(test)]
mod tests {
    use super::*;

    use std::collections::HashSet;
    use std::time::Duration;

    use commons::v2;

    use crate::avatar::Vehicle;
    use crate::travel_duration::EdgeDuration;

    #[test]
    fn empty_bridge() {
        assert_eq!(
            Bridge::new(vec![], Vehicle::None, BridgeType::Built),
            Err(InvalidBridge::Empty)
        )
    }

    #[test]
    fn diagonal_bridge() {
        assert_eq!(
            Bridge::new(
                vec![EdgeDuration {
                    from: v2(0, 0),
                    to: v2(1, 1),
                    duration: Some(Duration::from_secs(0))
                }],
                Vehicle::None,
                BridgeType::Built
            ),
            Err(InvalidBridge::Diagonal)
        )
    }

    #[test]
    fn discontinuous_bridge() {
        assert_eq!(
            Bridge::new(
                vec![
                    EdgeDuration {
                        from: v2(0, 0),
                        to: v2(1, 0),
                        duration: Some(Duration::from_secs(0))
                    },
                    EdgeDuration {
                        from: v2(2, 0),
                        to: v2(3, 0),
                        duration: Some(Duration::from_secs(0))
                    }
                ],
                Vehicle::None,
                BridgeType::Built
            ),
            Err(InvalidBridge::Discontinuous)
        )
    }

    #[test]
    fn start() {
        let bridge = Bridge::new(
            vec![
                EdgeDuration {
                    from: v2(0, 0),
                    to: v2(1, 0),
                    duration: Some(Duration::from_secs(0)),
                },
                EdgeDuration {
                    from: v2(1, 0),
                    to: v2(2, 0),
                    duration: Some(Duration::from_secs(0)),
                },
            ],
            Vehicle::None,
            BridgeType::Built,
        )
        .unwrap();

        assert_eq!(bridge.start(), v2(0, 0));
    }

    #[test]
    fn end() {
        let bridge = Bridge::new(
            vec![
                EdgeDuration {
                    from: v2(0, 0),
                    to: v2(1, 0),
                    duration: Some(Duration::from_secs(0)),
                },
                EdgeDuration {
                    from: v2(1, 0),
                    to: v2(2, 0),
                    duration: Some(Duration::from_secs(0)),
                },
            ],
            Vehicle::None,
            BridgeType::Built,
        )
        .unwrap();

        assert_eq!(bridge.end(), v2(2, 0));
    }

    #[test]
    fn one_way_edges_from_start() {
        let bridge = Bridge::new(
            vec![
                EdgeDuration {
                    from: v2(0, 0),
                    to: v2(1, 0),
                    duration: Some(Duration::from_secs(1)),
                },
                EdgeDuration {
                    from: v2(1, 0),
                    to: v2(2, 0),
                    duration: Some(Duration::from_secs(2)),
                },
            ],
            Vehicle::None,
            BridgeType::Built,
        )
        .unwrap();

        assert_eq!(
            bridge.edges_one_way(&v2(0, 0)).collect::<Vec<_>>(),
            vec![
                EdgeDuration {
                    from: v2(0, 0),
                    to: v2(1, 0),
                    duration: Some(Duration::from_secs(1)),
                },
                EdgeDuration {
                    from: v2(1, 0),
                    to: v2(2, 0),
                    duration: Some(Duration::from_secs(2)),
                },
            ]
        );
    }

    #[test]
    fn one_way_edges_from_end() {
        let bridge = Bridge::new(
            vec![
                EdgeDuration {
                    from: v2(0, 0),
                    to: v2(1, 0),
                    duration: Some(Duration::from_secs(1)),
                },
                EdgeDuration {
                    from: v2(1, 0),
                    to: v2(2, 0),
                    duration: Some(Duration::from_secs(2)),
                },
            ],
            Vehicle::None,
            BridgeType::Built,
        )
        .unwrap();

        assert_eq!(
            bridge.edges_one_way(&v2(2, 0)).collect::<Vec<_>>(),
            vec![
                EdgeDuration {
                    from: v2(2, 0),
                    to: v2(1, 0),
                    duration: Some(Duration::from_secs(2)),
                },
                EdgeDuration {
                    from: v2(1, 0),
                    to: v2(0, 0),
                    duration: Some(Duration::from_secs(1)),
                },
            ]
        );
    }

    #[test]
    fn total_edge() {
        let bridge = Bridge::new(
            vec![
                EdgeDuration {
                    from: v2(0, 0),
                    to: v2(1, 0),
                    duration: Some(Duration::from_secs(0)),
                },
                EdgeDuration {
                    from: v2(1, 0),
                    to: v2(2, 0),
                    duration: Some(Duration::from_secs(0)),
                },
            ],
            Vehicle::None,
            BridgeType::Built,
        )
        .unwrap();

        assert_eq!(bridge.total_edge(), Edge::new(v2(0, 0), v2(2, 0)));
    }

    #[test]
    fn total_duration() {
        let bridge = Bridge::new(
            vec![
                EdgeDuration {
                    from: v2(0, 0),
                    to: v2(1, 0),
                    duration: Some(Duration::from_secs(1)),
                },
                EdgeDuration {
                    from: v2(1, 0),
                    to: v2(2, 0),
                    duration: Some(Duration::from_secs(2)),
                },
            ],
            Vehicle::None,
            BridgeType::Built,
        )
        .unwrap();

        assert_eq!(bridge.total_duration(), Duration::from_secs(3));
    }

    #[test]
    fn total_edge_durations() {
        let bridge = Bridge::new(
            vec![
                EdgeDuration {
                    from: v2(0, 0),
                    to: v2(1, 0),
                    duration: Some(Duration::from_secs(1)),
                },
                EdgeDuration {
                    from: v2(1, 0),
                    to: v2(2, 0),
                    duration: Some(Duration::from_secs(2)),
                },
            ],
            Vehicle::None,
            BridgeType::Built,
        )
        .unwrap();

        assert_eq!(
            bridge.total_edge_durations().collect::<HashSet<_>>(),
            hashset! {
                EdgeDuration {
                    from: v2(0, 0),
                    to: v2(2, 0),
                    duration: Some(Duration::from_secs(3)),
                },
                EdgeDuration {
                    from: v2(2, 0),
                    to: v2(0, 0),
                    duration: Some(Duration::from_secs(3)),
                }
            }
        );
    }

    #[test]
    fn get_lowest_duration_bridge() {
        // Given
        let edge = Edge::new(v2(0, 0), v2(1, 0));
        let bridge_1 = Bridge::new(
            vec![EdgeDuration {
                from: v2(0, 0),
                to: v2(1, 0),
                duration: Some(Duration::from_secs(1)),
            }],
            Vehicle::None,
            BridgeType::Built,
        )
        .unwrap();
        let bridge_2 = Bridge::new(
            vec![EdgeDuration {
                from: v2(0, 0),
                to: v2(1, 0),
                duration: Some(Duration::from_secs(2)),
            }],
            Vehicle::None,
            BridgeType::Built,
        )
        .unwrap();

        let bridges = hashmap! {
            edge => hashset!{bridge_1.clone(), bridge_2},
        };

        // Then
        assert_eq!(bridges.get_lowest_duration_bridge(&edge), Some(&bridge_1));
    }

    #[test]
    fn count_bridges() {
        // Given

        // Edge from (1, 0) with with multiple built bridges - counts as 1
        let edge_1 = Edge::new(v2(1, 0), v2(1, 1));
        let edge_1_bridge_1 = Bridge::new(
            vec![EdgeDuration {
                from: v2(1, 0),
                to: v2(1, 1),
                duration: Some(Duration::from_secs(1)),
            }],
            Vehicle::None,
            BridgeType::Built,
        )
        .unwrap();
        let edge_1_bridge_2 = Bridge::new(
            vec![EdgeDuration {
                from: v2(1, 0),
                to: v2(1, 1),
                duration: Some(Duration::from_secs(2)),
            }],
            Vehicle::None,
            BridgeType::Built,
        )
        .unwrap();

        // Edge from (1, 0) with theoretical bridge - not counted
        let edge_2 = Edge::new(v2(1, 0), v2(1, 2));
        let edge_2_bridge_1 = Bridge::new(
            vec![EdgeDuration {
                from: v2(1, 0),
                to: v2(1, 2),
                duration: Some(Duration::from_secs(1)),
            }],
            Vehicle::None,
            BridgeType::Theoretical,
        )
        .unwrap();

        // Edge not from or to (1, 0) - not counted
        let edge_3 = Edge::new(v2(1, 1), v2(1, 3));
        let edge_3_bridge_1 = Bridge::new(
            vec![EdgeDuration {
                from: v2(1, 1),
                to: v2(1, 3),
                duration: Some(Duration::from_secs(1)),
            }],
            Vehicle::None,
            BridgeType::Built,
        )
        .unwrap();

        // Edge to (1, 0) - counts as 1
        let edge_4 = Edge::new(v2(0, 0), v2(1, 0));
        let edge_4_bridge_1 = Bridge::new(
            vec![EdgeDuration {
                from: v2(0, 0),
                to: v2(1, 0),
                duration: Some(Duration::from_secs(1)),
            }],
            Vehicle::None,
            BridgeType::Built,
        )
        .unwrap();

        let bridges = hashmap! {
            edge_1 => hashset!{edge_1_bridge_1, edge_1_bridge_2},
            edge_2 => hashset!{edge_2_bridge_1},
            edge_3 => hashset!{edge_3_bridge_1},
            edge_4 => hashset!{edge_4_bridge_1}
        };

        // Then
        assert_eq!(bridges.count_bridges_at(&v2(1, 0), &BridgeType::Built), 2);
    }
}
