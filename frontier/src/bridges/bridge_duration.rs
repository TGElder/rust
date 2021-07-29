use std::convert::TryInto;
use std::time::Duration;

use commons::edge::Edge;

use crate::bridges::{Bridge, BridgeType, Pier, Segment};

pub struct BridgeTypeDurationFn {
    pub one_cell: Duration,
    pub penalty: Duration,
}

pub struct BridgeDurationFn {
    pub theoretical: BridgeTypeDurationFn,
    pub built: BridgeTypeDurationFn,
}

// TODO move Segment here
impl BridgeTypeDurationFn {
    fn segment_duration(&self, from: &Pier, to: &Pier) -> Duration {
        let length = Edge::new(from.position, to.position).length();

        self.one_cell * length.try_into().unwrap()
    }
}

impl BridgeDurationFn {
    fn total_duration(&self, bridge: &Bridge) -> Duration {
        let duration_fn = match bridge.bridge_type {
            BridgeType::Theoretical => &self.theoretical,
            BridgeType::Built => &self.built,
        };
        bridge
            .segments
            .iter()
            .map(|segment| duration_fn.segment_duration(&segment.from, &segment.to))
            .sum()
    }

    fn segments(&self, bridge: &Bridge) -> Vec<Segment> {
        // TODO iterator
        let duration_fn = match bridge.bridge_type {
            BridgeType::Theoretical => &self.theoretical,
            BridgeType::Built => &self.built,
        };
        bridge
            .segments
            .iter()
            .map(|segment| Segment {
                duration: duration_fn.segment_duration(&segment.from, &segment.to),
                ..*segment
            })
            .collect()
    }
}
