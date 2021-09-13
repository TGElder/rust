use std::convert::TryInto;
use std::iter::once;
use std::time::Duration;

use commons::edge::Edge;
use commons::V2;
use serde::{Deserialize, Serialize};

use crate::avatar::{AvatarLoad, Frame};
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

impl BridgeTypeDurationFn {
    fn segment_duration(&self, from: &Pier, to: &Pier) -> Duration {
        let length = Edge::new(from.position, to.position).length();

        self.one_cell * length.try_into().unwrap() + self.segment_penalty(from, to)
    }

    fn segment_penalty(&self, from: &Pier, to: &Pier) -> Duration {
        if from.vehicle != to.vehicle {
            self.penalty
        } else {
            Duration::from_secs(0)
        }
    }
}

impl BridgeDurationFn {
    pub fn total_duration(&self, bridge: &Bridge) -> Duration {
        let duration_fn = self.duration_fn(bridge);
        bridge
            .segments()
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

    pub fn frames_from(
        &self,
        bridge: &Bridge,
        from: &V2<usize>,
        start_at: &u128,
        load: AvatarLoad,
    ) -> Vec<Frame> {
        let mut arrival = *start_at;

        let mut out = Vec::with_capacity(bridge.piers.len());
        for (i, Segment { from, to }) in bridge.segments_from(from).enumerate() {
            if i == 0 {
                out.push(Frame {
                    position: from.position,
                    elevation: from.elevation,
                    arrival,
                    vehicle: from.vehicle,
                    rotation: from.rotation,
                    load,
                });
            }
            arrival += self
                .duration_fn(bridge)
                .segment_duration(&from, &to)
                .as_micros();
            out.push(Frame {
                position: to.position,
                elevation: to.elevation,
                arrival,
                vehicle: to.vehicle,
                rotation: to.rotation,
                load,
            });
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use commons::v2;

    use crate::avatar::{Rotation, Vehicle};

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
            piers: vec![
                Pier {
                    position: v2(0, 0),
                    elevation: 0.0,
                    platform: true,
                    rotation: Rotation::Up,
                    vehicle: Vehicle::None,
                },
                Pier {
                    position: v2(1, 0),
                    elevation: 1.0,
                    platform: true,
                    rotation: Rotation::Up,
                    vehicle: Vehicle::None,
                },
                Pier {
                    position: v2(3, 0),
                    elevation: 2.0,
                    platform: true,
                    rotation: Rotation::Up,
                    vehicle: Vehicle::None,
                },
            ],

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
    fn changing_vehicle_should_incur_penalty() {
        // Given
        let built_bridge = Bridge {
            piers: vec![
                Pier {
                    position: v2(0, 0),
                    elevation: 0.0,
                    platform: true,
                    rotation: Rotation::Up,
                    vehicle: Vehicle::None,
                },
                Pier {
                    position: v2(1, 0),
                    elevation: 1.0,
                    platform: true,
                    rotation: Rotation::Up,
                    vehicle: Vehicle::Boat,
                },
                Pier {
                    position: v2(3, 0),
                    elevation: 2.0,
                    platform: true,
                    rotation: Rotation::Up,
                    vehicle: Vehicle::None,
                },
            ],

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
            Duration::from_secs(3 + 2 * 2)
        );
        assert_eq!(
            duration_fn.total_duration(&theoretical_bridge),
            Duration::from_secs(3 * 3 + 4 * 2)
        );
    }

    #[test]
    fn total_edge_durations() {
        // Given
        let bridge = Bridge {
            piers: vec![
                Pier {
                    position: v2(0, 0),
                    elevation: 0.0,
                    platform: true,
                    rotation: Rotation::Up,
                    vehicle: Vehicle::None,
                },
                Pier {
                    position: v2(1, 0),
                    elevation: 1.0,
                    platform: true,
                    rotation: Rotation::Up,
                    vehicle: Vehicle::None,
                },
                Pier {
                    position: v2(2, 0),
                    elevation: 2.0,
                    platform: true,
                    rotation: Rotation::Up,
                    vehicle: Vehicle::None,
                },
            ],

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
            piers: vec![
                Pier {
                    position: v2(0, 0),
                    elevation: 0.0,
                    platform: true,
                    rotation: Rotation::Up,
                    vehicle: Vehicle::None,
                },
                Pier {
                    position: v2(1, 0),
                    elevation: 1.0,
                    platform: true,
                    rotation: Rotation::Up,
                    vehicle: Vehicle::None,
                },
                Pier {
                    position: v2(3, 0),
                    elevation: 2.0,
                    platform: true,
                    rotation: Rotation::Up,
                    vehicle: Vehicle::None,
                },
            ],

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
    fn frames_from() {
        // Given
        let bridge = Bridge {
            piers: vec![
                Pier {
                    position: v2(0, 0),
                    elevation: 0.0,
                    platform: true,
                    rotation: Rotation::Up,
                    vehicle: Vehicle::None,
                },
                Pier {
                    position: v2(1, 0),
                    elevation: 1.0,
                    platform: true,
                    rotation: Rotation::Up,
                    vehicle: Vehicle::Boat,
                },
                Pier {
                    position: v2(2, 0),
                    elevation: 2.0,
                    platform: true,
                    rotation: Rotation::Up,
                    vehicle: Vehicle::Boat,
                },
            ],

            bridge_type: BridgeType::Built,
        };

        let duration_fn = bridge_duration_fn();

        // When
        let actual = duration_fn.frames_from(&bridge, &v2(2, 0), &11, AvatarLoad::None);

        // Then
        assert_eq!(
            actual,
            vec![
                Frame {
                    position: v2(2, 0),
                    elevation: 2.0,
                    arrival: 11,
                    vehicle: Vehicle::Boat,
                    rotation: Rotation::Down,
                    load: AvatarLoad::None,
                },
                Frame {
                    position: v2(1, 0),
                    elevation: 1.0,
                    arrival: 1_000_011,
                    vehicle: Vehicle::Boat,
                    rotation: Rotation::Down,
                    load: AvatarLoad::None,
                },
                Frame {
                    position: v2(0, 0),
                    elevation: 0.0,
                    arrival: 4_000_011, // With penalty for vehicle change
                    vehicle: Vehicle::None,
                    rotation: Rotation::Down,
                    load: AvatarLoad::None,
                }
            ]
        );
    }
}
