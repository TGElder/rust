use super::*;

use crate::settlement::Settlement;
use commons::edge::Edge;
use commons::V2;

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub enum Build {
    Road(Edge),
    Town(Settlement),
    Crops { position: V2<usize>, rotated: bool },
}

#[derive(Clone, Debug, Eq, Hash, PartialEq, Deserialize, Serialize)]
pub enum BuildKey {
    Road(Edge),
    Settlement(V2<usize>),
    Crops(V2<usize>),
}

impl Build {
    pub fn key(&self) -> BuildKey {
        match self {
            Build::Road(edge) => BuildKey::Road(*edge),
            Build::Town(Settlement { position, .. }) => BuildKey::Settlement(*position),
            Build::Crops { position, .. } => BuildKey::Crops(*position),
        }
    }
}

#[cfg(test)]
mod tests {
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
    fn settlement_build_key() {
        // Given
        let position = v2(1, 2);
        let settlement = Settlement {
            position,
            ..Settlement::default()
        };
        let build = Build::Town(settlement);

        // Then
        assert_eq!(build.key(), BuildKey::Settlement(position));
    }

    #[test]
    fn crops_build_key() {
        // Given
        let position = v2(1, 2);
        let build = Build::Crops {
            position,
            rotated: true,
        };

        // Then
        assert_eq!(build.key(), BuildKey::Crops(position));
    }
}
