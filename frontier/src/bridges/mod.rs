mod bridge_duration;

pub use bridge_duration::{BridgeDurationFn, BridgeTypeDurationFn, TimedSegment};

use commons::V2;
use serde::{Deserialize, Serialize};

use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::{error, fmt};

use commons::edge::Edge;

use crate::avatar::Vehicle;

#[derive(Debug, Clone, Eq, Hash, PartialEq, Deserialize, Serialize)]
pub struct Bridge {
    pub piers: Vec<Pier>,
    pub vehicle: Vehicle,
    pub bridge_type: BridgeType,
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

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct Segment<'a> {
    pub from: &'a Pier,
    pub to: &'a Pier,
}

#[derive(Debug, Clone, Eq, Hash, PartialEq, Deserialize, Serialize)]
pub enum BridgeType {
    Theoretical,
    Built,
}

pub type Bridges = HashMap<Edge, HashSet<Bridge>>;

impl Bridge {
    pub fn validate(self) -> Result<Bridge, InvalidBridge> {
        self.validate_piers()?;

        Ok(self)
    }

    pub fn start(&self) -> &Pier {
        self.piers.first().unwrap()
    }

    pub fn end(&self) -> &Pier {
        self.piers.last().unwrap()
    }

    pub fn total_edge(&self) -> Edge {
        Edge::new(self.start().position, self.end().position)
    }

    pub fn segments(&self) -> impl Iterator<Item = Segment> {
        let from = self.piers.iter();
        let to = self.piers.iter().skip(1);
        from.zip(to).map(|(from, to)| Segment { from, to })
    }

    pub fn segments_rev(&self) -> impl Iterator<Item = Segment> {
        let from = self.piers.iter().rev();
        let to = self.piers.iter().rev().skip(1);
        from.zip(to).map(|(from, to)| Segment { from, to })
    }

    fn validate_piers(&self) -> Result<(), InvalidBridge> {
        let piers = &self.piers;
        let first = unwrap_or!(piers.first(), return Err(InvalidBridge::Empty));
        let last = piers.last().unwrap();

        if Edge::new_safe(first.position, last.position).is_err() {
            return Err(InvalidBridge::Diagonal);
        }

        if self
            .segments()
            .map(|segment| Edge::new_safe(segment.from.position, segment.to.position))
            .any(|result| result.is_err())
        {
            return Err(InvalidBridge::DiagonalSegment);
        }

        Ok(())
    }
}

impl<'a> Segment<'a> {
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
            .flat_map(|bridge| bridge.piers.iter())
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
        }
    }
}

impl error::Error for InvalidBridge {}

#[cfg(test)]
mod tests {
    use super::*;

    use commons::v2;

    use crate::avatar::Vehicle;

    #[test]
    fn empty_bridge() {
        assert_eq!(
            Bridge {
                piers: vec![],
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
                piers: vec![
                    Pier {
                        position: v2(0, 0),
                        elevation: 0.0,
                        platform: true,
                    },
                    Pier {
                        position: v2(1, 1),
                        elevation: 0.0,
                        platform: true,
                    },
                ],
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
                piers: vec![
                    Pier {
                        position: v2(0, 0),
                        elevation: 0.0,
                        platform: true,
                    },
                    Pier {
                        position: v2(1, 0),
                        elevation: 0.0,
                        platform: true,
                    },
                    Pier {
                        position: v2(0, 1),
                        elevation: 0.0,
                        platform: true,
                    },
                ],
                vehicle: Vehicle::None,
                bridge_type: BridgeType::Built
            }
            .validate(),
            Err(InvalidBridge::DiagonalSegment)
        )
    }

    #[test]
    fn start() {
        let bridge = Bridge {
            piers: vec![
                Pier {
                    position: v2(0, 0),
                    elevation: 0.0,
                    platform: true,
                },
                Pier {
                    position: v2(1, 0),
                    elevation: 1.0,
                    platform: true,
                },
                Pier {
                    position: v2(2, 0),
                    elevation: 2.0,
                    platform: true,
                },
            ],
            vehicle: Vehicle::None,
            bridge_type: BridgeType::Built,
        };

        assert_eq!(
            *bridge.start(),
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
            piers: vec![
                Pier {
                    position: v2(0, 0),
                    elevation: 0.0,
                    platform: true,
                },
                Pier {
                    position: v2(1, 0),
                    elevation: 1.0,
                    platform: true,
                },
                Pier {
                    position: v2(2, 0),
                    elevation: 2.0,
                    platform: true,
                },
            ],
            vehicle: Vehicle::None,
            bridge_type: BridgeType::Built,
        };

        assert_eq!(
            *bridge.end(),
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
            piers: vec![
                Pier {
                    position: v2(0, 0),
                    elevation: 0.0,
                    platform: true,
                },
                Pier {
                    position: v2(1, 0),
                    elevation: 1.0,
                    platform: true,
                },
                Pier {
                    position: v2(2, 0),
                    elevation: 2.0,
                    platform: true,
                },
            ],
            vehicle: Vehicle::None,
            bridge_type: BridgeType::Built,
        };

        assert_eq!(bridge.total_edge(), Edge::new(v2(0, 0), v2(2, 0)));
    }

    #[test]
    fn segments() {
        let bridge = Bridge {
            piers: vec![
                Pier {
                    position: v2(0, 0),
                    elevation: 0.0,
                    platform: true,
                },
                Pier {
                    position: v2(1, 0),
                    elevation: 1.0,
                    platform: true,
                },
                Pier {
                    position: v2(2, 0),
                    elevation: 2.0,
                    platform: true,
                },
            ],
            vehicle: Vehicle::None,
            bridge_type: BridgeType::Built,
        };

        assert_eq!(
            bridge.segments().collect::<Vec<_>>(),
            vec![
                Segment {
                    from: &bridge.piers[0],
                    to: &bridge.piers[1],
                },
                Segment {
                    from: &bridge.piers[1],
                    to: &bridge.piers[2],
                },
            ]
        );
    }

    #[test]
    fn segments_rev() {
        let bridge = Bridge {
            piers: vec![
                Pier {
                    position: v2(0, 0),
                    elevation: 0.0,
                    platform: true,
                },
                Pier {
                    position: v2(1, 0),
                    elevation: 1.0,
                    platform: true,
                },
                Pier {
                    position: v2(2, 0),
                    elevation: 2.0,
                    platform: true,
                },
            ],
            vehicle: Vehicle::None,
            bridge_type: BridgeType::Built,
        };

        assert_eq!(
            bridge.segments_rev().collect::<Vec<_>>(),
            vec![
                Segment {
                    from: &bridge.piers[2],
                    to: &bridge.piers[1],
                },
                Segment {
                    from: &bridge.piers[1],
                    to: &bridge.piers[0],
                },
            ]
        );
    }

    #[test]
    fn segment_edge() {
        let from = Pier {
            position: v2(1, 0),
            elevation: 0.0,
            platform: true,
        };
        let to = Pier {
            position: v2(0, 0),
            elevation: 0.0,
            platform: true,
        };
        let segment = Segment {
            from: &from,
            to: &to,
        };

        assert_eq!(segment.edge(), Edge::new(v2(0, 0), v2(1, 0)));
    }

    #[test]
    fn count_platforms_at_counts_platforms_in_different_edges() {
        // Given
        let edge_1 = Edge::new(v2(1, 0), v2(1, 1));
        let bridge_1 = Bridge {
            piers: vec![
                Pier {
                    position: v2(1, 0),
                    elevation: 0.0,
                    platform: true,
                },
                Pier {
                    position: v2(1, 1),
                    elevation: 0.0,
                    platform: true,
                },
            ],
            vehicle: Vehicle::None,
            bridge_type: BridgeType::Built,
        };

        let edge_2 = Edge::new(v2(1, 0), v2(2, 0));
        let bridge_2 = Bridge {
            piers: vec![
                Pier {
                    position: v2(1, 0),
                    elevation: 0.0,
                    platform: true,
                },
                Pier {
                    position: v2(2, 0),
                    elevation: 0.0,
                    platform: true,
                },
            ],
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
            piers: vec![
                Pier {
                    position: v2(0, 0),
                    elevation: 0.0,
                    platform: true,
                },
                Pier {
                    position: v2(1, 0),
                    elevation: 0.0,
                    platform: true,
                },
            ],
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
            piers: vec![
                Pier {
                    position: v2(0, 0),
                    elevation: 0.0,
                    platform: true,
                },
                Pier {
                    position: v2(2, 0),
                    elevation: 0.0,
                    platform: true,
                },
            ],
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
            piers: vec![
                Pier {
                    position: v2(1, 0),
                    elevation: 0.0,
                    platform: false,
                },
                Pier {
                    position: v2(2, 0),
                    elevation: 0.0,
                    platform: true,
                },
            ],
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
            piers: vec![
                Pier {
                    position: v2(1, 0),
                    elevation: 0.0,
                    platform: true,
                },
                Pier {
                    position: v2(2, 0),
                    elevation: 0.0,
                    platform: true,
                },
            ],
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
