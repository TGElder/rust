use super::*;

use crate::bridge::Bridge;
use crate::resource::Mine;
use crate::settlement::Settlement;
use commons::edge::Edge;
use commons::V2;

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub enum Build {
    Road(Edge),
    Bridge(Bridge),
    Town(Settlement),
    Mine { position: V2<usize>, mine: Mine },
}

#[derive(Clone, Debug, Eq, Hash, PartialEq, Deserialize, Serialize)]
pub enum BuildKey {
    Road(Edge),
    Bridge(Bridge),
    Town(V2<usize>),
    Mine(V2<usize>),
}

impl Build {
    pub fn key(&self) -> BuildKey {
        match self {
            Build::Road(edge) => BuildKey::Road(*edge),
            Build::Bridge(bridge) => BuildKey::Bridge(bridge.clone()),
            Build::Town(Settlement { position, .. }) => BuildKey::Town(*position),
            Build::Mine { position, .. } => BuildKey::Mine(*position),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::avatar::Vehicle;
    use crate::bridge::BridgeType;
    use crate::travel_duration::EdgeDuration;

    use super::*;

    use commons::v2;

    #[test]
    fn road_build_key() {
        // Given
        let edge = Edge::new(v2(1, 2), v2(1, 3));
        let build = Build::Road(edge);

        // Then
        assert_eq!(build.key(), BuildKey::Road(edge));
    }

    #[test]
    fn bridge_build_key() {
        // Given
        let bridge = Bridge::new(
            vec![
                EdgeDuration {
                    from: v2(0, 0),
                    to: v2(1, 0),
                    duration: Some(Duration::from_millis(1)),
                },
                EdgeDuration {
                    from: v2(1, 0),
                    to: v2(2, 0),
                    duration: Some(Duration::from_millis(2)),
                },
            ],
            Vehicle::None,
            BridgeType::Built,
        )
        .unwrap();
        let build = Build::Bridge(bridge.clone());

        // Then
        assert_eq!(build.key(), BuildKey::Bridge(bridge));
    }

    #[test]
    fn town_build_key() {
        // Given
        let position = v2(1, 2);
        let town = Settlement {
            position,
            ..Settlement::default()
        };
        let build = Build::Town(town);

        // Then
        assert_eq!(build.key(), BuildKey::Town(position));
    }

    #[test]
    fn mine_build_key() {
        // Given
        let position = v2(1, 2);
        let build = Build::Mine {
            position,
            mine: Mine::Crop,
        };

        // Then
        assert_eq!(build.key(), BuildKey::Mine(position));
    }
}
