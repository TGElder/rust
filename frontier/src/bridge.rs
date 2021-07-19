use commons::V2;
use serde::{Deserialize, Serialize};

use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::iter::once;
use std::time::Duration;
use std::{error, fmt};

use commons::edge::Edge;

use crate::avatar::Vehicle;
use crate::travel_duration::EdgeDuration;

#[derive(Debug, Clone, Eq, Hash, PartialEq, Deserialize, Serialize)]
pub struct Bridge {
    segments: Vec<Segment>,
    vehicle: Vehicle,
    bridge_type: BridgeType,
}

#[derive(Debug, Clone, Eq, Hash, PartialEq, Deserialize, Serialize)]
pub struct Segment {
    pub from: Pier,
    pub to: Pier,
    pub duration: Duration,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct Pier {
    pub position: V2<usize>,
    pub elevation: f32,
}

impl Hash for Pier {
    fn hash<H>(&self, state: &mut H)
    where
        H: Hasher,
    {
        self.position.hash(state);
    }
}

impl PartialEq for Pier {
    fn eq(&self, other: &Self) -> bool {
        self.position == other.position
    }
}

impl Eq for Pier {}

#[derive(Debug, Clone, Eq, Hash, PartialEq, Deserialize, Serialize)]
pub enum BridgeType {
    Theoretical,
    Built,
}

pub type Bridges = HashMap<Edge, HashSet<Bridge>>;

impl Bridge {
    pub fn new(
        segments: Vec<Segment>,
        vehicle: Vehicle,
        bridge_type: BridgeType,
    ) -> Result<Bridge, InvalidBridge> {
        Self::validate_segments(&segments)?;
        Ok(Bridge {
            segments,
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

    pub fn start(&self) -> Pier {
        self.segments.first().unwrap().from
    }

    pub fn end(&self) -> Pier {
        self.segments.last().unwrap().to
    }

    pub fn segments_one_way<'a>(
        &'a self,
        from: &V2<usize>,
    ) -> Box<dyn Iterator<Item = Segment> + 'a> {
        if self.start().position == *from {
            Box::new(self.segments.iter().cloned())
        } else if self.end().position == *from {
            Box::new(self.segments_reversed())
        } else {
            panic!(
                "Position {} is at neither end of the bridge {:?}!",
                from, self.segments
            );
        }
    }

    #[allow(clippy::needless_lifetimes)] // https://github.com/rust-lang/rust-clippy/issues/5787
    fn segments_reversed<'a>(&'a self) -> impl Iterator<Item = Segment> + 'a {
        self.segments
            .iter()
            .map(|segment| Segment {
                from: segment.to,
                to: segment.from,
                duration: segment.duration,
            })
            .rev()
    }

    pub fn total_edge(&self) -> Edge {
        Edge::new(self.start().position, self.end().position)
    }

    pub fn total_duration(&self) -> Duration {
        self.segments.iter().map(|edge| edge.duration).sum()
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

    fn validate_segments(segments: &[Segment]) -> Result<(), InvalidBridge> {
        let first = unwrap_or!(segments.first(), return Err(InvalidBridge::Empty));
        let last = segments.last().unwrap();

        if Edge::new_safe(first.from.position, last.to.position).is_err() {
            return Err(InvalidBridge::Diagonal);
        }

        if segments
            .iter()
            .map(|segment| Edge::new_safe(segment.from.position, segment.to.position))
            .any(|result| result.is_err())
        {
            return Err(InvalidBridge::DiagonalSegment);
        }

        let next = segments.iter().skip(1);
        segments.iter().zip(next).try_for_each(|(a, b)| {
            if a.to.position != b.from.position {
                Err(InvalidBridge::Discontinuous)
            } else {
                Ok(())
            }
        })
    }
}

impl Segment {
    pub fn edge(&self) -> Edge {
        Edge::new(self.from.position, self.to.position)
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
    DiagonalSegment,
    Discontinuous,
}

impl fmt::Display for InvalidBridge {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InvalidBridge::Empty => write!(f, "Bridge must have at least one segment"),
            InvalidBridge::Diagonal => {
                write!(f, "Bridge start and end must have same x or y coordinate")
            }
            InvalidBridge::DiagonalSegment => {
                write!(f, "Bridge segments must not be diagonal")
            }
            InvalidBridge::Discontinuous => write!(f, "Bridge segments are not continuous"),
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
                vec![Segment {
                    from: Pier {
                        position: v2(0, 0),
                        elevation: 0.0,
                    },
                    to: Pier {
                        position: v2(1, 1),
                        elevation: 0.0,
                    },
                    duration: Duration::from_secs(0)
                }],
                Vehicle::None,
                BridgeType::Built
            ),
            Err(InvalidBridge::Diagonal)
        )
    }

    #[test]
    fn diagonal_segment_bridge() {
        assert_eq!(
            Bridge::new(
                vec![
                    Segment {
                        from: Pier {
                            position: v2(0, 0),
                            elevation: 0.0,
                        },
                        to: Pier {
                            position: v2(1, 0),
                            elevation: 0.0,
                        },
                        duration: Duration::from_secs(0)
                    },
                    Segment {
                        from: Pier {
                            position: v2(1, 0),
                            elevation: 0.0,
                        },
                        to: Pier {
                            position: v2(0, 1),
                            elevation: 0.0,
                        },
                        duration: Duration::from_secs(0)
                    }
                ],
                Vehicle::None,
                BridgeType::Built
            ),
            Err(InvalidBridge::DiagonalSegment)
        )
    }

    #[test]
    fn discontinuous_bridge() {
        assert_eq!(
            Bridge::new(
                vec![
                    Segment {
                        from: Pier {
                            position: v2(0, 0),
                            elevation: 0.0,
                        },
                        to: Pier {
                            position: v2(1, 0),
                            elevation: 0.0,
                        },
                        duration: Duration::from_secs(0)
                    },
                    Segment {
                        from: Pier {
                            position: v2(2, 0),
                            elevation: 0.0,
                        },
                        to: Pier {
                            position: v2(3, 0),
                            elevation: 0.0,
                        },
                        duration: Duration::from_secs(0)
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
                Segment {
                    from: Pier {
                        position: v2(0, 0),
                        elevation: 0.0,
                    },
                    to: Pier {
                        position: v2(1, 0),
                        elevation: 1.0,
                    },
                    duration: Duration::from_secs(0),
                },
                Segment {
                    from: Pier {
                        position: v2(1, 0),
                        elevation: 1.0,
                    },
                    to: Pier {
                        position: v2(2, 0),
                        elevation: 2.0,
                    },
                    duration: Duration::from_secs(0),
                },
            ],
            Vehicle::None,
            BridgeType::Built,
        )
        .unwrap();

        assert_eq!(
            bridge.start(),
            Pier {
                position: v2(0, 0),
                elevation: 0.0,
            }
        );
    }

    #[test]
    fn end() {
        let bridge = Bridge::new(
            vec![
                Segment {
                    from: Pier {
                        position: v2(0, 0),
                        elevation: 0.0,
                    },
                    to: Pier {
                        position: v2(1, 0),
                        elevation: 1.0,
                    },
                    duration: Duration::from_secs(0),
                },
                Segment {
                    from: Pier {
                        position: v2(1, 0),
                        elevation: 1.0,
                    },
                    to: Pier {
                        position: v2(2, 0),
                        elevation: 2.0,
                    },
                    duration: Duration::from_secs(0),
                },
            ],
            Vehicle::None,
            BridgeType::Built,
        )
        .unwrap();

        assert_eq!(
            bridge.end(),
            Pier {
                position: v2(2, 0),
                elevation: 2.0,
            }
        );
    }

    #[test]
    fn segments_one_way_from_start() {
        let bridge = Bridge::new(
            vec![
                Segment {
                    from: Pier {
                        position: v2(0, 0),
                        elevation: 0.0,
                    },
                    to: Pier {
                        position: v2(1, 0),
                        elevation: 1.0,
                    },
                    duration: Duration::from_secs(1),
                },
                Segment {
                    from: Pier {
                        position: v2(1, 0),
                        elevation: 1.0,
                    },
                    to: Pier {
                        position: v2(2, 0),
                        elevation: 2.0,
                    },
                    duration: Duration::from_secs(2),
                },
            ],
            Vehicle::None,
            BridgeType::Built,
        )
        .unwrap();

        assert_eq!(
            bridge.segments_one_way(&v2(0, 0)).collect::<Vec<_>>(),
            vec![
                Segment {
                    from: Pier {
                        position: v2(0, 0),
                        elevation: 0.0,
                    },
                    to: Pier {
                        position: v2(1, 0),
                        elevation: 1.0,
                    },
                    duration: Duration::from_secs(1),
                },
                Segment {
                    from: Pier {
                        position: v2(1, 0),
                        elevation: 1.0,
                    },
                    to: Pier {
                        position: v2(2, 0),
                        elevation: 2.0,
                    },
                    duration: Duration::from_secs(2),
                },
            ]
        );
    }

    #[test]
    fn segments_one_way_from_end() {
        let bridge = Bridge::new(
            vec![
                Segment {
                    from: Pier {
                        position: v2(0, 0),
                        elevation: 0.0,
                    },
                    to: Pier {
                        position: v2(1, 0),
                        elevation: 1.0,
                    },
                    duration: Duration::from_secs(1),
                },
                Segment {
                    from: Pier {
                        position: v2(1, 0),
                        elevation: 1.0,
                    },
                    to: Pier {
                        position: v2(2, 0),
                        elevation: 2.0,
                    },
                    duration: Duration::from_secs(2),
                },
            ],
            Vehicle::None,
            BridgeType::Built,
        )
        .unwrap();

        assert_eq!(
            bridge.segments_one_way(&v2(2, 0)).collect::<Vec<_>>(),
            vec![
                Segment {
                    from: Pier {
                        position: v2(2, 0),
                        elevation: 2.0,
                    },
                    to: Pier {
                        position: v2(1, 0),
                        elevation: 1.0,
                    },
                    duration: Duration::from_secs(2),
                },
                Segment {
                    from: Pier {
                        position: v2(1, 0),
                        elevation: 1.0,
                    },
                    to: Pier {
                        position: v2(0, 0),
                        elevation: 0.0,
                    },
                    duration: Duration::from_secs(1),
                },
            ]
        );
    }

    #[test]
    fn total_edge() {
        let bridge = Bridge::new(
            vec![
                Segment {
                    from: Pier {
                        position: v2(0, 0),
                        elevation: 0.0,
                    },
                    to: Pier {
                        position: v2(1, 0),
                        elevation: 0.0,
                    },
                    duration: Duration::from_secs(0),
                },
                Segment {
                    from: Pier {
                        position: v2(1, 0),
                        elevation: 0.0,
                    },
                    to: Pier {
                        position: v2(2, 0),
                        elevation: 0.0,
                    },
                    duration: Duration::from_secs(0),
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
                Segment {
                    from: Pier {
                        position: v2(0, 0),
                        elevation: 0.0,
                    },
                    to: Pier {
                        position: v2(1, 0),
                        elevation: 0.0,
                    },
                    duration: Duration::from_secs(1),
                },
                Segment {
                    from: Pier {
                        position: v2(1, 0),
                        elevation: 0.0,
                    },
                    to: Pier {
                        position: v2(2, 0),
                        elevation: 0.0,
                    },
                    duration: Duration::from_secs(2),
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
                Segment {
                    from: Pier {
                        position: v2(0, 0),
                        elevation: 0.0,
                    },
                    to: Pier {
                        position: v2(1, 0),
                        elevation: 0.0,
                    },
                    duration: Duration::from_secs(1),
                },
                Segment {
                    from: Pier {
                        position: v2(1, 0),
                        elevation: 0.0,
                    },
                    to: Pier {
                        position: v2(2, 0),
                        elevation: 0.0,
                    },
                    duration: Duration::from_secs(2),
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
    fn segment_edge() {
        let segment = Segment {
            from: Pier {
                position: v2(1, 0),
                elevation: 0.0,
            },
            to: Pier {
                position: v2(0, 0),
                elevation: 0.0,
            },
            duration: Duration::from_secs(0),
        };

        assert_eq!(segment.edge(), Edge::new(v2(0, 0), v2(1, 0)));
    }

    #[test]
    fn get_lowest_duration_bridge() {
        // Given
        let edge = Edge::new(v2(0, 0), v2(1, 0));
        let bridge_1 = Bridge::new(
            vec![Segment {
                from: Pier {
                    position: v2(0, 0),
                    elevation: 0.0,
                },
                to: Pier {
                    position: v2(1, 0),
                    elevation: 0.0,
                },
                duration: Duration::from_secs(1),
            }],
            Vehicle::None,
            BridgeType::Built,
        )
        .unwrap();
        let bridge_2 = Bridge::new(
            vec![Segment {
                from: Pier {
                    position: v2(0, 0),
                    elevation: 0.0,
                },
                to: Pier {
                    position: v2(1, 0),
                    elevation: 0.0,
                },
                duration: Duration::from_secs(2),
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
            vec![Segment {
                from: Pier {
                    position: v2(1, 0),
                    elevation: 0.0,
                },
                to: Pier {
                    position: v2(1, 1),
                    elevation: 0.0,
                },
                duration: Duration::from_secs(1),
            }],
            Vehicle::None,
            BridgeType::Built,
        )
        .unwrap();
        let edge_1_bridge_2 = Bridge::new(
            vec![Segment {
                from: Pier {
                    position: v2(1, 0),
                    elevation: 0.0,
                },
                to: Pier {
                    position: v2(1, 1),
                    elevation: 0.0,
                },
                duration: Duration::from_secs(2),
            }],
            Vehicle::None,
            BridgeType::Built,
        )
        .unwrap();

        // Edge from (1, 0) with theoretical bridge - not counted
        let edge_2 = Edge::new(v2(1, 0), v2(1, 2));
        let edge_2_bridge_1 = Bridge::new(
            vec![Segment {
                from: Pier {
                    position: v2(1, 0),
                    elevation: 0.0,
                },
                to: Pier {
                    position: v2(1, 2),
                    elevation: 0.0,
                },
                duration: Duration::from_secs(1),
            }],
            Vehicle::None,
            BridgeType::Theoretical,
        )
        .unwrap();

        // Edge not from or to (1, 0) - not counted
        let edge_3 = Edge::new(v2(1, 1), v2(1, 3));
        let edge_3_bridge_1 = Bridge::new(
            vec![Segment {
                from: Pier {
                    position: v2(1, 1),
                    elevation: 0.0,
                },
                to: Pier {
                    position: v2(1, 3),
                    elevation: 0.0,
                },
                duration: Duration::from_secs(1),
            }],
            Vehicle::None,
            BridgeType::Built,
        )
        .unwrap();

        // Edge to (1, 0) - counts as 1
        let edge_4 = Edge::new(v2(0, 0), v2(1, 0));
        let edge_4_bridge_1 = Bridge::new(
            vec![Segment {
                from: Pier {
                    position: v2(0, 0),
                    elevation: 0.0,
                },
                to: Pier {
                    position: v2(1, 0),
                    elevation: 0.0,
                },
                duration: Duration::from_secs(1),
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
