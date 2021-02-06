use super::*;

use std::default::Default;

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct State {
    pub params: SimulationParams,
    pub instructions: Vec<Instruction>,
}

impl Default for State {
    fn default() -> State {
        State {
            params: SimulationParams::default(),
            instructions: vec![],
        }
    }
}
