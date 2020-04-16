use commons::V2;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct Citizen {
    pub name: String,
    pub birthday: u128,
    pub birthplace: V2<usize>,
    pub farm: Option<V2<usize>>,
}
