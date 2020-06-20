use super::*;

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub enum Instruction {
    Step,
    Town(V2<usize>),
}
