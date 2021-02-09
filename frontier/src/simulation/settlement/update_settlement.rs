use commons::async_trait::async_trait;
use commons::V2;

use crate::simulation::settlement::instruction::Instruction;
use crate::simulation::settlement::processor::Processor;
use crate::simulation::settlement::state::State;
use crate::traits::has::HasParameters;
use crate::traits::{
    Controlled, GetSettlement, SendRoutes, SendSettlements,
    UpdateSettlement as UpdateSettlementTrait, UpdateTerritory, VisibleLandPositions,
    WithRouteToPorts, WithTraffic,
};

pub struct UpdateSettlement<T> {
    pub tx: T,
}

impl<T> UpdateSettlement<T> {
    pub fn new(tx: T) -> UpdateSettlement<T> {
        UpdateSettlement { tx }
    }
}

#[async_trait]
impl<T> Processor for UpdateSettlement<T>
where
    T: Controlled
        + HasParameters
        + GetSettlement
        + SendRoutes
        + SendSettlements
        + UpdateSettlementTrait
        + UpdateTerritory
        + VisibleLandPositions
        + WithRouteToPorts
        + WithTraffic
        + Send
        + Sync,
{
    async fn process(&mut self, mut state: State, instruction: &Instruction) -> State {
        let mut instructions = match instruction {
            Instruction::UpdateHomelandPopulation(position) => {
                self.update_homeland_at(position).await
            }
            Instruction::GetTerritory(position) => self.update_town_at(position).await,
            _ => vec![],
        };
        state.instructions.append(&mut instructions);
        state
    }
}

impl<T> UpdateSettlement<T>
where
    T: Controlled
        + HasParameters
        + GetSettlement
        + SendRoutes
        + SendSettlements
        + UpdateSettlementTrait
        + UpdateTerritory
        + VisibleLandPositions
        + WithRouteToPorts
        + WithTraffic,
{
    async fn update_homeland_at(&self, homeland: &V2<usize>) -> Vec<Instruction> {
        let settlement = unwrap_or!(self.tx.get_settlement(*homeland).await, return vec![]);
        self.update_homeland(&settlement).await;

        vec![Instruction::UpdateCurrentPopulation(*homeland)]
    }

    async fn update_town_at(&self, town: &V2<usize>) -> Vec<Instruction> {
        let settlement = unwrap_or!(self.tx.get_settlement(*town).await, return vec![]);
        let territory = self.get_territory(town).await;
        let traffic = self.get_town_traffic(&territory).await;
        self.update_town(&settlement, &traffic).await;

        vec![Instruction::UpdateCurrentPopulation(*town)]
    }
}
