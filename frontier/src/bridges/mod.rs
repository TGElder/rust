mod bridge_duration;

pub use bridge_duration::{BridgeDurationFn, BridgeTypeDurationFn};

use commons::V2;
use serde::{Deserialize, Serialize};

use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::iter::once;
use std::time::Duration;
use std::{error, fmt};

use commons::edge::Edge;

use crate::avatar::Vehicle;

#[derive(Debug, Clone, Eq, Hash, PartialEq, Deserialize, Serialize)]
pub struct Bridge {
    pub segments: Vec<Segment>,
    pub vehicle: Vehicle,
    pub bridge_type: BridgeType,
}

#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq, Deserialize, Serialize)]
pub struct Segment {
    pub from: Pier,
    pub to: Pier,
    pub duration: Duration,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct Pier {
    pub position: V2<usize>,
    pub elevation: f32,
    pub platform: bool,
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
    pub fn validate(self) -> Result<Bridge, InvalidBridge> {
        self.validate_segments()?;

        Ok(self)
    }

    pub fn start(&self) -> Pier {
        self.segments.first().unwrap().from
    }

    pub fn end(&self) -> Pier {
        self.segments.last().unwrap().to
    }

    pub fn total_edge(&self) -> Edge {
        Edge::new(self.start().position, self.end().position)
    }

    fn validate_segments(&self) -> Result<(), InvalidBridge> {
        let segments = &self.segments;
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
    fn count_platforms_at(&self, position: &V2<usize>, bridge_type: &BridgeType) -> usize;
}

impl BridgesExt for Bridges {
    fn count_platforms_at(&self, position: &V2<usize>, bridge_type: &BridgeType) -> usize {
        self.iter()
            .flat_map(|(_, bridges)| bridges.iter())
            .filter(|bridge| bridge.bridge_type == *bridge_type)
            .flat_map(|bridge| bridge.segments.iter())
            .flat_map(|segment| once(segment.from).chain(once(segment.to)))
            .filter(|pier| pier.platform)
            .filter(|pier| pier.position == *position)
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

    use std::time::Duration;

    use commons::v2;

    use crate::avatar::Vehicle;

    #[test]
    fn empty_bridge() {
        assert_eq!(
            Bridge {
                segments: vec![],
                vehicle: Vehicle::None,
                bridge_type: BridgeType::Built,
            }
            .validate(),
            Err(InvalidBridge::Empty)
        )
    }

    #[test]
    fn diagonal_bridge() {
        assert_eq!(
            Bridge {
                segments: vec![Segment {
                    from: Pier {
                        position: v2(0, 0),
                        elevation: 0.0,
                        platform: true,
                    },
                    to: Pier {
                        position: v2(1, 1),
                        elevation: 0.0,
                        platform: true,
                    },
                    duration: Duration::from_secs(0)
                }],
                vehicle: Vehicle::None,
                bridge_type: BridgeType::Built
            }
            .validate(),
            Err(InvalidBridge::Diagonal)
        )
    }

    #[test]
    fn diagonal_segment_bridge() {
        assert_eq!(
            Bridge {
                segments: vec![
                    Segment {
                        from: Pier {
                            position: v2(0, 0),
                            elevation: 0.0,
                            platform: true,
                        },
                        to: Pier {
                            position: v2(1, 0),
                            elevation: 0.0,
                            platform: true,
                        },
                        duration: Duration::from_secs(0)
                    },
                    Segment {
                        from: Pier {
                            position: v2(1, 0),
                            elevation: 0.0,
                            platform: true,
                        },
                        to: Pier {
                            position: v2(0, 1),
                            elevation: 0.0,
                            platform: true,
                        },
                        duration: Duration::from_secs(0)
                    }
                ],
                vehicle: Vehicle::None,
                bridge_type: BridgeType::Built
            }
            .validate(),
            Err(InvalidBridge::DiagonalSegment)
        )
    }

    #[test]
    fn discontinuous_bridge() {
        assert_eq!(
            Bridge {
                segments: vec![
                    Segment {
                        from: Pier {
                            position: v2(0, 0),
                            elevation: 0.0,
                            platform: true,
                        },
                        to: Pier {
                            position: v2(1, 0),
                            elevation: 0.0,
                            platform: true,
                        },
                        duration: Duration::from_secs(0)
                    },
                    Segment {
                        from: Pier {
                            position: v2(2, 0),
                            elevation: 0.0,
                            platform: true,
                        },
                        to: Pier {
                            position: v2(3, 0),
                            elevation: 0.0,
                            platform: true,
                        },
                        duration: Duration::from_secs(0)
                    }
                ],
                vehicle: Vehicle::None,
                bridge_type: BridgeType::Built
            }
            .validate(),
            Err(InvalidBridge::Discontinuous)
        )
    }

    #[test]
    fn start() {
        let bridge = Bridge {
            segments: vec![
                Segment {
                    from: Pier {
                        position: v2(0, 0),
                        elevation: 0.0,
                        platform: true,
                    },
                    to: Pier {
                        position: v2(1, 0),
                        elevation: 1.0,
                        platform: true,
                    },
                    duration: Duration::from_secs(0),
                },
                Segment {
                    from: Pier {
                        position: v2(1, 0),
                        elevation: 1.0,
                        platform: true,
                    },
                    to: Pier {
                        position: v2(2, 0),
                        elevation: 2.0,
                        platform: true,
                    },
                    duration: Duration::from_secs(0),
                },
            ],
            vehicle: Vehicle::None,
            bridge_type: BridgeType::Built,
        };

        assert_eq!(
            bridge.start(),
            Pier {
                position: v2(0, 0),
                elevation: 0.0,
                platform: true,
            }
        );
    }

    #[test]
    fn end() {
        let bridge = Bridge {
            segments: vec![
                Segment {
                    from: Pier {
                        position: v2(0, 0),
                        elevation: 0.0,
                        platform: true,
                    },
                    to: Pier {
                        position: v2(1, 0),
                        elevation: 1.0,
                        platform: true,
                    },
                    duration: Duration::from_secs(0),
                },
                Segment {
                    from: Pier {
                        position: v2(1, 0),
                        elevation: 1.0,
                        platform: true,
                    },
                    to: Pier {
                        position: v2(2, 0),
                        elevation: 2.0,
                        platform: true,
                    },
                    duration: Duration::from_secs(0),
                },
            ],
            vehicle: Vehicle::None,
            bridge_type: BridgeType::Built,
        };

        assert_eq!(
            bridge.end(),
            Pier {
                position: v2(2, 0),
                elevation: 2.0,
                platform: true,
            }
        );
    }

    #[test]
    fn total_edge() {
        let bridge = Bridge {
            segments: vec![
                Segment {
                    from: Pier {
                        position: v2(0, 0),
                        elevation: 0.0,
                        platform: true,
                    },
                    to: Pier {
                        position: v2(1, 0),
                        elevation: 1.0,
                        platform: true,
                    },
                    duration: Duration::from_secs(0),
                },
                Segment {
                    from: Pier {
                        position: v2(1, 0),
                        elevation: 1.0,
                        platform: true,
                    },
                    to: Pier {
                        position: v2(2, 0),
                        elevation: 2.0,
                        platform: true,
                    },
                    duration: Duration::from_secs(0),
                },
            ],
            vehicle: Vehicle::None,
            bridge_type: BridgeType::Built,
        };

        assert_eq!(bridge.total_edge(), Edge::new(v2(0, 0), v2(2, 0)));
    }

    #[test]
    fn segment_edge() {
        let segment = Segment {
            from: Pier {
                position: v2(1, 0),
                elevation: 0.0,
                platform: true,
            },
            to: Pier {
                position: v2(0, 0),
                elevation: 0.0,
                platform: true,
            },
            duration: Duration::from_secs(0),
        };

        assert_eq!(segment.edge(), Edge::new(v2(0, 0), v2(1, 0)));
    }

    #[test]
    fn count_platforms_at_counts_multiple_platforms_against_same_edge() {
        // Given
        let edge_1 = Edge::new(v2(1, 0), v2(1, 1));
        let bridge_1 = Bridge {
            segments: vec![Segment {
                from: Pier {
                    position: v2(1, 0),
                    elevation: 0.0,
                    platform: true,
                },
                to: Pier {
                    position: v2(1, 1),
                    elevation: 0.0,
                    platform: true,
                },
                duration: Duration::from_secs(1),
            }],
            vehicle: Vehicle::None,
            bridge_type: BridgeType::Built,
        };
        let bridge_2 = Bridge {
            segments: vec![Segment {
                from: Pier {
                    position: v2(1, 0),
                    elevation: 0.0,
                    platform: true,
                },
                to: Pier {
                    position: v2(1, 1),
                    elevation: 0.0,
                    platform: true,
                },
                duration: Duration::from_secs(2),
            }],
            vehicle: Vehicle::None,
            bridge_type: BridgeType::Built,
        };

        let bridges = hashmap! {
            edge_1 => hashset!{bridge_1, bridge_2},
        };

        // Then
        assert_eq!(bridges.count_platforms_at(&v2(1, 0), &BridgeType::Built), 2);
    }

    #[test]
    fn count_platforms_at_counts_platforms_in_different_edges() {
        // Given
        let edge_1 = Edge::new(v2(1, 0), v2(1, 1));
        let bridge_1 = Bridge {
            segments: vec![Segment {
                from: Pier {
                    position: v2(1, 0),
                    elevation: 0.0,
                    platform: true,
                },
                to: Pier {
                    position: v2(1, 1),
                    elevation: 0.0,
                    platform: true,
                },
                duration: Duration::from_secs(1),
            }],
            vehicle: Vehicle::None,
            bridge_type: BridgeType::Built,
        };

        let edge_2 = Edge::new(v2(1, 0), v2(2, 0));
        let bridge_2 = Bridge {
            segments: vec![Segment {
                from: Pier {
                    position: v2(1, 0),
                    elevation: 0.0,
                    platform: true,
                },
                to: Pier {
                    position: v2(2, 0),
                    elevation: 0.0,
                    platform: true,
                },
                duration: Duration::from_secs(2),
            }],
            vehicle: Vehicle::None,
            bridge_type: BridgeType::Built,
        };

        let bridges = hashmap! {
            edge_1 => hashset!{bridge_1},
            edge_2 => hashset!{bridge_2},
        };

        // Then
        assert_eq!(bridges.count_platforms_at(&v2(1, 0), &BridgeType::Built), 2);
    }

    #[test]
    fn count_platforms_at_counts_platform_in_to_pier() {
        // Given
        let edge_1 = Edge::new(v2(0, 0), v2(1, 0));
        let bridge_1 = Bridge {
            segments: vec![Segment {
                from: Pier {
                    position: v2(0, 0),
                    elevation: 0.0,
                    platform: true,
                },
                to: Pier {
                    position: v2(1, 0),
                    elevation: 0.0,
                    platform: true,
                },
                duration: Duration::from_secs(1),
            }],
            vehicle: Vehicle::None,
            bridge_type: BridgeType::Built,
        };

        let bridges = hashmap! {
            edge_1 => hashset!{bridge_1},
        };

        // Then
        assert_eq!(bridges.count_platforms_at(&v2(1, 0), &BridgeType::Built), 1);
    }

    #[test]
    fn count_platforms_does_not_count_platforms_in_other_positions() {
        // Given
        let edge_1 = Edge::new(v2(0, 0), v2(2, 0));
        let bridge_1 = Bridge {
            segments: vec![Segment {
                from: Pier {
                    position: v2(0, 0),
                    elevation: 0.0,
                    platform: true,
                },
                to: Pier {
                    position: v2(2, 0),
                    elevation: 0.0,
                    platform: true,
                },
                duration: Duration::from_secs(1),
            }],
            vehicle: Vehicle::None,
            bridge_type: BridgeType::Built,
        };

        let bridges = hashmap! {
            edge_1 => hashset!{bridge_1},
        };

        // Then
        assert_eq!(bridges.count_platforms_at(&v2(1, 0), &BridgeType::Built), 0);
    }

    #[test]
    fn count_platforms_does_not_count_piers_where_platform_is_false() {
        // Given
        let edge_1 = Edge::new(v2(1, 0), v2(2, 0));
        let bridge_1 = Bridge {
            segments: vec![Segment {
                from: Pier {
                    position: v2(1, 0),
                    elevation: 0.0,
                    platform: false,
                },
                to: Pier {
                    position: v2(2, 0),
                    elevation: 0.0,
                    platform: true,
                },
                duration: Duration::from_secs(1),
            }],
            vehicle: Vehicle::None,
            bridge_type: BridgeType::Built,
        };

        let bridges = hashmap! {
            edge_1 => hashset!{bridge_1},
        };

        // Then
        assert_eq!(bridges.count_platforms_at(&v2(1, 0), &BridgeType::Built), 0);
    }

    #[test]
    fn count_platforms_does_not_count_bridges_of_different_type() {
        // Given
        let edge_1 = Edge::new(v2(1, 0), v2(2, 0));
        let bridge_1 = Bridge {
            segments: vec![Segment {
                from: Pier {
                    position: v2(1, 0),
                    elevation: 0.0,
                    platform: true,
                },
                to: Pier {
                    position: v2(2, 0),
                    elevation: 0.0,
                    platform: true,
                },
                duration: Duration::from_secs(1),
            }],
            vehicle: Vehicle::None,
            bridge_type: BridgeType::Theoretical,
        };

        let bridges = hashmap! {
            edge_1 => hashset!{bridge_1},
        };

        // Then
        assert_eq!(bridges.count_platforms_at(&v2(1, 0), &BridgeType::Built), 0);
    }
}
