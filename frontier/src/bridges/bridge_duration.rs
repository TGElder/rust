use std::convert::TryInto;
use std::iter::once;
use std::time::Duration;

use commons::edge::Edge;
use commons::V2;
use serde::{Deserialize, Serialize};

use crate::bridges::{Bridge, BridgeType, Pier, Segment};
use crate::travel_duration::EdgeDuration;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct BridgeDurationFn {
    pub theoretical: BridgeTypeDurationFn,
    pub built: BridgeTypeDurationFn,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct BridgeTypeDurationFn {
    pub one_cell: Duration,
    pub penalty: Duration,
}

impl Default for BridgeDurationFn {
    fn default() -> Self {
        BridgeDurationFn {
            built: BridgeTypeDurationFn {
                one_cell: Duration::from_secs(1),
                penalty: Duration::from_secs(1),
            },
            theoretical: BridgeTypeDurationFn {
                one_cell: Duration::from_secs(1),
                penalty: Duration::from_secs(1),
            },
        }
    }
}

// TODO move Segment here
impl BridgeTypeDurationFn {
    fn segment_duration(&self, from: &Pier, to: &Pier) -> Duration {
        let length = Edge::new(from.position, to.position).length();

        self.one_cell * length.try_into().unwrap()
    }
}

impl BridgeDurationFn {
    pub fn total_duration(&self, bridge: &Bridge) -> Duration {
        let duration_fn = self.duration_fn(bridge);
        bridge
            .segments
            .iter()
            .map(|segment| duration_fn.segment_duration(&segment.from, &segment.to))
            .sum()
    }

    fn duration_fn(&self, bridge: &Bridge) -> &BridgeTypeDurationFn {
        match bridge.bridge_type {
            BridgeType::Theoretical => &self.theoretical,
            BridgeType::Built => &self.built,
        }
    }

    #[allow(clippy::needless_lifetimes)] // https://github.com/rust-lang/rust-clippy/issues/5787
    pub fn total_edge_durations<'a>(
        &'a self,
        bridge: &'a Bridge,
    ) -> impl Iterator<Item = EdgeDuration> + 'a {
        let edge = bridge.total_edge();
        let duration = self.total_duration(bridge);
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

    pub fn lowest_duration_bridge<'a, I>(&self, bridges: I) -> Option<&'a Bridge>
    where
        I: IntoIterator<Item = &'a Bridge>,
    {
        bridges
            .into_iter()
            .min_by_key(|bridge| self.total_duration(bridge))
    }

    pub fn segments_one_way<'a>(
        &'a self,
        bridge: &'a Bridge,
        from: &V2<usize>,
    ) -> Box<dyn Iterator<Item = Segment> + 'a> {
        let duration_fn = self.duration_fn(bridge);
        if bridge.start().position == *from {
            Box::new(bridge.segments.iter().map(move |segment| Segment {
                from: segment.from,
                to: segment.to,
                duration: duration_fn.segment_duration(&segment.from, &segment.to),
            }))
        } else if bridge.end().position == *from {
            Box::new(
                bridge
                    .segments
                    .iter()
                    .map(move |segment| Segment {
                        from: segment.to,
                        to: segment.from,
                        duration: duration_fn.segment_duration(&segment.to, &segment.from),
                    })
                    .rev(),
            )
        } else {
            panic!(
                "Position {} is at neither end of the bridge {:?}!",
                from, bridge.segments
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use commons::v2;

    use crate::avatar::Vehicle;
    use crate::bridges::Segment;

    use super::*;

    fn bridge_duration_fn() -> BridgeDurationFn {
        BridgeDurationFn {
            built: BridgeTypeDurationFn {
                one_cell: Duration::from_secs(1),
                penalty: Duration::from_secs(2),
            },
            theoretical: BridgeTypeDurationFn {
                one_cell: Duration::from_secs(3),
                penalty: Duration::from_secs(4),
            },
        }
    }

    #[test]
    fn total_duration() {
        // Given
        let built_bridge = Bridge {
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
                        position: v2(3, 0),
                        elevation: 2.0,
                        platform: true,
                    },
                    duration: Duration::from_secs(0),
                },
            ],
            vehicle: Vehicle::None,
            bridge_type: BridgeType::Built,
        };

        let theoretical_bridge = Bridge {
            bridge_type: BridgeType::Theoretical,
            ..built_bridge.clone()
        };

        let duration_fn = bridge_duration_fn();

        // Then
        assert_eq!(
            duration_fn.total_duration(&built_bridge),
            Duration::from_secs(3)
        );
        assert_eq!(
            duration_fn.total_duration(&theoretical_bridge),
            Duration::from_secs(3 * 3)
        );
    }

    #[test]
    fn total_edge_durations() {
        // Given
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
                    duration: Duration::from_secs(1),
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
                    duration: Duration::from_secs(2),
                },
            ],
            vehicle: Vehicle::None,
            bridge_type: BridgeType::Theoretical,
        };

        let duration_fn = bridge_duration_fn();

        // Then
        assert_eq!(
            duration_fn
                .total_edge_durations(&bridge)
                .collect::<HashSet<_>>(),
            hashset! {
                EdgeDuration {
                    from: v2(0, 0),
                    to: v2(2, 0),
                    duration: Some(Duration::from_secs(3 * 2)),
                },
                EdgeDuration {
                    from: v2(2, 0),
                    to: v2(0, 0),
                    duration: Some(Duration::from_secs(3 * 2)),
                }
            }
        );
    }

    #[test]
    fn lowest_duration_bridge() {
        // Given
        let built_bridge = Bridge {
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
                        position: v2(3, 0),
                        elevation: 2.0,
                        platform: true,
                    },
                    duration: Duration::from_secs(0),
                },
            ],
            vehicle: Vehicle::None,
            bridge_type: BridgeType::Built,
        };

        let theoretical_bridge = Bridge {
            bridge_type: BridgeType::Theoretical,
            ..built_bridge.clone()
        };

        let duration_fn = bridge_duration_fn();

        // Then
        assert_eq!(
            duration_fn.lowest_duration_bridge(&[built_bridge.clone(), theoretical_bridge]),
            Some(&built_bridge)
        );
    }

    #[test]
    fn segments_one_way_from_start() {
        // Given
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
                    duration: Duration::from_secs(1),
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
                    duration: Duration::from_secs(1),
                },
            ],
            vehicle: Vehicle::None,
            bridge_type: BridgeType::Built,
        };

        let duration_fn = bridge_duration_fn();

        // Then
        assert_eq!(
            duration_fn
                .segments_one_way(&bridge, &v2(0, 0))
                .collect::<Vec<_>>(),
            vec![
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
                    duration: Duration::from_secs(1),
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
                    duration: Duration::from_secs(1),
                },
            ]
        );
    }

    #[test]
    fn segments_one_way_from_end() {
        // Given
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
                    duration: Duration::from_secs(1),
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
                    duration: Duration::from_secs(1),
                },
            ],
            vehicle: Vehicle::None,
            bridge_type: BridgeType::Built,
        };

        let duration_fn = bridge_duration_fn();

        // When
        assert_eq!(
            duration_fn
                .segments_one_way(&bridge, &v2(2, 0))
                .collect::<Vec<_>>(),
            vec![
                Segment {
                    from: Pier {
                        position: v2(2, 0),
                        elevation: 2.0,
                        platform: true,
                    },
                    to: Pier {
                        position: v2(1, 0),
                        elevation: 1.0,
                        platform: true,
                    },
                    duration: Duration::from_secs(1),
                },
                Segment {
                    from: Pier {
                        position: v2(1, 0),
                        elevation: 1.0,
                        platform: true,
                    },
                    to: Pier {
                        position: v2(0, 0),
                        elevation: 0.0,
                        platform: true,
                    },
                    duration: Duration::from_secs(1),
                },
            ]
        );
    }
}
