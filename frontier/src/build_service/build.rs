use super::*;

use crate::settlement::Settlement;
use commons::V2;

#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub enum Build {
    Road(V2<usize>),
    Settlement(Settlement),
}
