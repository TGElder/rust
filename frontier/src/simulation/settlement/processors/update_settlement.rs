use commons::async_trait::async_trait;
use commons::V2;

use crate::simulation::settlement::instruction::Instruction;
use crate::simulation::settlement::processor::Processor;
use crate::simulation::settlement::state::State;
use crate::traits::{Controlled, GetSettlement, UpdateTerritory};

use super::processors::GetTerritory;

pub struct UpdateSettlement<T> {
    pub tx: T,
    pub get_territory: GetTerritory<T>,
}

#[async_trait]
impl<T> Processor for UpdateSettlement<T>
where
    T: Controlled + GetSettlement + UpdateTerritory + Send + Sync,
{
    async fn process(&mut self, mut state: State, instruction: &Instruction) -> State {
        let mut instructions = match instruction {
            Instruction::GetTerritory(position) => self.update_town(position).await,
            _ => vec![],
        };
        state.instructions.append(&mut instructions);
        state
    }
}

impl<T> UpdateSettlement<T>
where
    T: Controlled + GetSettlement + UpdateTerritory,
{
    async fn update_town(&self, town: &V2<usize>) -> Vec<Instruction> {
        let settlement = unwrap_or!(self.tx.get_settlement(*town).await, return vec![]);
        let territory = self.get_territory.get_territory(town).await;

        vec![Instruction::GetTownTraffic {
            settlement,
            territory,
        }]
    }
}
