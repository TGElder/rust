use super::*;

use crate::resource::Mine;
use crate::settlement::Settlement;
use commons::edge::Edge;
use commons::V2;

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub enum Build {
    Road(Edge),
    Town(Settlement),
    Mine { position: V2<usize>, mine: Mine },
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Deserialize, Serialize)]
pub enum BuildKey {
    Road(Edge),
    Town(V2<usize>),
    Mine(V2<usize>),
}

impl Build {
    pub fn key(&self) -> BuildKey {
        match self {
            Build::Road(edge) => BuildKey::Road(*edge),
            Build::Town(Settlement { position, .. }) => BuildKey::Town(*position),
            Build::Mine { position, .. } => BuildKey::Mine(*position),
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
    fn object_build_key() {
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
