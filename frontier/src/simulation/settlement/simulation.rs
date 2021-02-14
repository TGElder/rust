use std::sync::Arc;

use commons::async_trait::async_trait;
use commons::log::debug;
use commons::process::Step;
use commons::V2;
use futures::future::join_all;

use crate::avatar::AvatarTravelModeFn;
use crate::settlement::{Settlement, SettlementClass};
use crate::simulation::settlement::demand::Demand;
use crate::simulation::settlement::model::Routes;
use crate::traits::has::HasParameters;
use crate::traits::{
    ClosestTargetsWithPlannedRoads, Controlled, GetSettlement, InBoundsWithPlannedRoads,
    LowestDurationWithoutPlannedRoads, Micros, RefreshEdges, RefreshPositions, RemoveTown,
    SendRoutes, SendSettlements, SendWorld, UpdateSettlement as UpdateSettlementTrait,
    UpdateTerritory, VisibleLandPositions, WithEdgeTraffic, WithRouteToPorts, WithSimQueue,
    WithTraffic,
};

use super::demand::demand_fn::{homeland_demand_fn, town_demand_fn};

pub struct SettlementSimulation<T> {
    pub(super) tx: T,
    pub(super) homeland_demand_fn: fn(&Settlement) -> Vec<Demand>,
    pub(super) town_demand_fn: fn(&Settlement) -> Vec<Demand>,
}

impl<T> SettlementSimulation<T> {
    pub fn new(tx: T) -> SettlementSimulation<T> {
        SettlementSimulation {
            tx,
            homeland_demand_fn,
            town_demand_fn,
        }
    }
}

#[async_trait]
impl<T> Step for SettlementSimulation<T>
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
        + SendWorld
        + UpdateSettlementTrait
        + UpdateTerritory
        + VisibleLandPositions
        + WithEdgeTraffic
        + WithRouteToPorts
        + WithSimQueue
        + WithTraffic
        + Send
        + Sync,
{
    async fn step(&mut self) {
        let position = self.tx.mut_sim_queue(|sim_queue| sim_queue.pop()).await;

        match position {
            Some(position) => self.update_settlement_at(&position).await,
            None => self.replenish_sim_queue().await,
        }
    }
}

impl<T> SettlementSimulation<T>
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
        + SendWorld
        + UpdateSettlementTrait
        + UpdateTerritory
        + VisibleLandPositions
        + WithEdgeTraffic
        + WithRouteToPorts
        + WithSimQueue
        + WithTraffic,
{
    async fn update_settlement_at(&self, position: &V2<usize>) {
        let settlement = unwrap_or!(self.tx.get_settlement(*position).await, return);
        debug!(
            "{:?} {} -> {}",
            settlement.name, settlement.current_population, settlement.target_population
        );
        match settlement.class {
            SettlementClass::Homeland => self.update_homeland_settlement(settlement).await,
            SettlementClass::Town => self.update_town_settlement(settlement).await,
        }
    }

    async fn update_homeland_settlement(&self, settlement: Settlement) {
        self.update_homeland(&settlement).await;
        let updated = self.update_current_population(settlement).await;
        let demand = (self.homeland_demand_fn)(&updated);
        self.get_all_route_changes(demand).await
    }

    async fn update_town_settlement(&self, settlement: Settlement) {
        let territory = self.get_territory(&settlement.position).await;
        let traffic = self.get_town_traffic(&territory).await;
        join!(
            self.update_town(&settlement, &traffic),
            self.remove_town(&settlement, &traffic), // TODO should be after population update
        );
        let updated = self.update_current_population(settlement).await;
        let demand = (self.town_demand_fn)(&updated);
        self.get_all_route_changes(demand).await
    }

    async fn get_all_route_changes(&self, demand: Vec<Demand>) {
        let futures = demand
            .into_iter()
            .map(|demand| self.process_demand(demand))
            .collect::<Vec<_>>();
        join_all(futures).await;
    }

    async fn process_demand(&self, demand: Demand) {
        let Routes { key, route_set } = self.get_routes(demand).await;
        let route_changes = self.update_routes_and_get_changes(key, route_set).await;
        let travel_mode_fn = Arc::new(AvatarTravelModeFn::new(
            self.tx.parameters().avatar_travel.min_navigable_river_width,
        )); // TODO find better way of passing this
        join!(
            self.update_edge_traffic(&route_changes),
            self.update_position_traffic(&route_changes),
            self.update_route_to_ports(&route_changes, travel_mode_fn),
        );
    }

    async fn replenish_sim_queue(&self) {
        let settlements = self
            .tx
            .send_settlements(|settlements| settlements.keys().copied().collect::<Vec<_>>())
            .await;
        self.tx
            .mut_sim_queue(move |sim_queue| {
                if sim_queue.is_empty() {
                    *sim_queue = settlements;
                }
            })
            .await;
    }
}
