use commons::M;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Visited {
    pub visited: M<bool>,
    pub all_visited: bool,
}
