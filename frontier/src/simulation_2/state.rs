use super::*;

use std::default::Default;

#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub struct State {
    pub instructions: Vec<Instruction>,
}

impl Default for State {
    fn default() -> State {
        State {
            instructions: vec![],
        }
    }
}