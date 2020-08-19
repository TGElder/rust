use super::*;

use crate::settlement::Settlement;
use commons::edge::Edge;
use commons::V2;

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub enum Build {
    Road(Edge),
    Settlement(Settlement),
    Crops { position: V2<usize>, rotated: bool },
}
