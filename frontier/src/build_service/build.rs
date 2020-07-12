use super::*;

use crate::settlement::Settlement;
use commons::V2;

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub enum Build {
    Road(V2<usize>),
    Settlement {
        candidate_positions: Vec<V2<usize>>,
        settlement: Settlement,
    },
}
