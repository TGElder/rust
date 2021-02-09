use commons::async_trait::async_trait;
use commons::V2;

use crate::simulation::settlement::instruction::Instruction;
use crate::simulation::settlement::processor::Processor;
use crate::simulation::settlement::processors::{GetTownTraffic, UpdateTown};
use crate::simulation::settlement::state::State;
use crate::traits::has::HasParameters;
use crate::traits::{
    Controlled, GetSettlement, SendRoutes, SendSettlements,
    UpdateSettlement as UpdateSettlementTrait, UpdateTerritory, VisibleLandPositions,
    WithRouteToPorts, WithTraffic,
};

use super::processors::GetTerritory;
use super::UpdateHomeland;

pub struct UpdateSettlement<T> {
    pub tx: T,
    pub get_territory: GetTerritory<T>,
    pub get_town_traffic: GetTownTraffic<T>,
    pub update_homeland: UpdateHomeland<T>,
    pub update_town: UpdateTown<T>,
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
            Instruction::UpdateHomelandPopulation(position) => self.update_homeland(position).await,
            Instruction::GetTerritory(position) => self.update_town(position).await,
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
    async fn update_homeland(&self, homeland: &V2<usize>) -> Vec<Instruction> {
        let settlement = unwrap_or!(self.tx.get_settlement(*homeland).await, return vec![]);
        self.update_homeland.update_homeland(&settlement).await;

        vec![Instruction::UpdateCurrentPopulation(*homeland)]
    }

    async fn update_town(&self, town: &V2<usize>) -> Vec<Instruction> {
        let settlement = unwrap_or!(self.tx.get_settlement(*town).await, return vec![]);
        let territory = self.get_territory.get_territory(town).await;
        let traffic = self.get_town_traffic.get_town_traffic(&territory).await;
        self.update_town.update_town(&settlement, &traffic).await;

        vec![Instruction::UpdateCurrentPopulation(*town)]
    }
}
