use commons::async_trait::async_trait;
use commons::V2;
use futures::future::join_all;

use crate::settlement::Settlement;
use crate::simulation::settlement::demand::Demand;
use crate::simulation::settlement::demand_fn::homeland_demand_fn;
use crate::simulation::settlement::instruction::{Instruction, Routes};
use crate::simulation::settlement::processor::Processor;
use crate::simulation::settlement::state::State;
use crate::traits::has::HasParameters;
use crate::traits::{
    ClosestTargetsWithPlannedRoads, Controlled, GetSettlement, InBoundsWithPlannedRoads,
    LowestDurationWithoutPlannedRoads, Micros, RefreshEdges, RefreshPositions, RemoveTown,
    SendRoutes, SendSettlements, UpdateSettlement as UpdateSettlementTrait, UpdateTerritory,
    VisibleLandPositions, WithEdgeTraffic, WithRouteToPorts, WithTraffic,
};

use super::demand_fn::town_demand_fn;

pub struct UpdateSettlement<T> {
    pub(super) tx: T,
    pub(super) homeland_demand_fn: fn(&Settlement) -> Vec<Demand>,
    pub(super) town_demand_fn: fn(&Settlement) -> Vec<Demand>,
}

impl<T> UpdateSettlement<T> {
    pub fn new(tx: T) -> UpdateSettlement<T> {
        UpdateSettlement {
            tx,
            homeland_demand_fn,
            town_demand_fn,
        }
    }
}

#[async_trait]
impl<T> Processor for UpdateSettlement<T>
where
    T: ClosestTargetsWithPlannedRoads
        + Controlled
        + HasParameters
        + InBoundsWithPlannedRoads
        + GetSettlement
        + LowestDurationWithoutPlannedRoads
        + Micros
        + RefreshEdges
        + RefreshPositions
        + RemoveTown
        + SendRoutes
        + SendSettlements
        + UpdateSettlementTrait
        + UpdateTerritory
        + VisibleLandPositions
        + WithEdgeTraffic
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
    T: ClosestTargetsWithPlannedRoads
        + Controlled
        + HasParameters
        + InBoundsWithPlannedRoads
        + GetSettlement
        + LowestDurationWithoutPlannedRoads
        + Micros
        + RefreshEdges
        + RefreshPositions
        + RemoveTown
        + SendRoutes
        + SendSettlements
        + UpdateSettlementTrait
        + UpdateTerritory
        + VisibleLandPositions
        + WithEdgeTraffic
        + WithRouteToPorts
        + WithTraffic,
{
    async fn update_homeland_at(&self, position: &V2<usize>) -> Vec<Instruction> {
        let settlement = unwrap_or!(self.tx.get_settlement(*position).await, return vec![]);
        self.update_homeland(&settlement).await;
        if let Some(updated) = self.update_current_population(*position).await {
            let demand = (self.homeland_demand_fn)(&updated);
            self.get_all_route_changes(demand).await
        } else {
            vec![]
        }
    }

    async fn update_town_at(&self, position: &V2<usize>) -> Vec<Instruction> {
        let settlement = unwrap_or!(self.tx.get_settlement(*position).await, return vec![]);
        let territory = self.get_territory(position).await;
        let traffic = self.get_town_traffic(&territory).await;
        join!(
            self.update_town(&settlement, &traffic),
            self.remove_town(&settlement, &traffic), // TODO should be after population update
        );
        if let Some(updated) = self.update_current_population(*position).await {
            let demand = (self.town_demand_fn)(&updated);
            self.get_all_route_changes(demand).await
        } else {
            vec![]
        }
    }

    async fn get_all_route_changes(&self, demand: Vec<Demand>) -> Vec<Instruction> {
        let futures = demand
            .into_iter()
            .map(|demand| self.get_route_changes(demand))
            .collect::<Vec<_>>();
        join_all(futures).await
    }

    async fn get_route_changes(&self, demand: Demand) -> Instruction {
        let Routes { key, route_set } = self.get_routes(demand).await;
        let route_changes = self.update_routes_and_get_changes(key, route_set).await;
        self.update_edge_traffic(&route_changes).await;
        Instruction::ProcessRouteChanges(route_changes)
    }
}
