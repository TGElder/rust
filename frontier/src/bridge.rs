use commons::V2;
use serde::{Deserialize, Serialize};

use std::collections::HashMap;
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

pub type Bridges = HashMap<Edge, Bridge>;

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
    fn edge() {
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

        assert_eq!(bridge.edge(), Edge::new(v2(0, 0), v2(2, 0)));
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
            bridge.one_way_edges(&v2(0, 0)).collect::<Vec<_>>(),
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
            bridge.one_way_edges(&v2(2, 0)).collect::<Vec<_>>(),
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
    fn both_way_edges() {
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
            bridge.both_way_edges().collect::<HashSet<_>>(),
            hashset! {
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
                EdgeDuration {
                    from: v2(2, 0),
                    to: v2(1, 0),
                    duration: Some(Duration::from_secs(2)),
                },
                EdgeDuration {
                    from: v2(1, 0),
                    to: v2(0, 0),
                    duration: Some(Duration::from_secs(1)),
                }
            }
        );
    }

    #[test]
    fn duration() {
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

        assert_eq!(bridge.duration(), Duration::from_secs(3));
    }
}
