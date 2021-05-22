use std::sync::Arc;

use commons::async_trait::async_trait;
use commons::log::debug;
use commons::process::Step;
use commons::V2;
use futures::future::join_all;

use crate::avatar::AvatarTravelModeFn;
use crate::settlement::{Settlement, SettlementClass};
use crate::simulation::settlement::demand::Demand;
use crate::simulation::settlement::model::{RouteChange, Routes};
use crate::traits::has::HasParameters;
use crate::traits::{
    ClosestTargetsForRoutes, Controlled, CostOfPath, GetSettlement, InBoundsForRoutes, Micros,
    RefreshEdges, RefreshPositions, RemoveTown, UpdateSettlement as UpdateSettlementTrait,
    UpdateTerritory, VisibleLandPositions, WithEdgeTraffic, WithRouteToPorts, WithRoutes,
    WithSettlements, WithSimQueue, WithTraffic, WithWorld,
};
use crate::travel::TravelDuration;

use super::demand::demand_fn::{homeland_demand_fn, town_demand_fn};

pub struct SettlementSimulation<T, D> {
    pub(super) cx: T,
    pub(super) homeland_demand_fn: fn(&Settlement) -> Vec<Demand>,
    pub(super) town_demand_fn: fn(&Settlement) -> Vec<Demand>,
    pub(super) travel_duration: Arc<D>,
}

impl<T, D> SettlementSimulation<T, D> {
    pub fn new(cx: T, travel_duration: Arc<D>) -> SettlementSimulation<T, D> {
        SettlementSimulation {
            cx,
            homeland_demand_fn,
            town_demand_fn,
            travel_duration,
        }
    }
}

#[async_trait]
impl<T, D> Step for SettlementSimulation<T, D>
where
    T: ClosestTargetsForRoutes
        + Controlled
        + CostOfPath
        + GetSettlement
        + HasParameters
        + InBoundsForRoutes
        + Micros
        + RefreshEdges
        + RefreshPositions
        + RemoveTown
        + UpdateSettlementTrait
        + UpdateTerritory
        + VisibleLandPositions
        + WithEdgeTraffic
        + WithRoutes
        + WithRouteToPorts
        + WithSettlements
        + WithSimQueue
        + WithTraffic
        + WithWorld
        + Send
        + Sync,
    D: TravelDuration,
{
    async fn step(&mut self) {
        let position = self.cx.mut_sim_queue(|sim_queue| sim_queue.pop()).await;

        match position {
            Some(position) => self.update_settlement_at(&position).await,
            None => self.replenish_sim_queue().await,
        }
    }
}

impl<T, D> SettlementSimulation<T, D>
where
    T: ClosestTargetsForRoutes
        + Controlled
        + CostOfPath
        + GetSettlement
        + HasParameters
        + InBoundsForRoutes
        + Micros
        + RefreshEdges
        + RefreshPositions
        + RemoveTown
        + WithRoutes
        + WithSettlements
        + UpdateSettlementTrait
        + UpdateTerritory
        + VisibleLandPositions
        + WithEdgeTraffic
        + WithRouteToPorts
        + WithSimQueue
        + WithTraffic
        + WithWorld,
    D: TravelDuration,
{
    async fn update_settlement_at(&self, position: &V2<usize>) {
        let settlement = unwrap_or!(self.cx.get_settlement(position).await, return);
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
        let settlement = self.update_homeland(settlement).await;
        let settlement = self.update_current_population(settlement).await;
        let demand = (self.homeland_demand_fn)(&settlement);
        self.cx.update_settlement(settlement).await;
        self.get_all_route_changes(demand).await
    }

    async fn update_town_settlement(&self, settlement: Settlement) {
        let territory = self.get_territory(&settlement.position).await;
        let traffic = self.get_town_traffic(&territory).await;
        let settlement = self.update_town(settlement, &traffic).await;
        let settlement = self.update_current_population(settlement).await;
        if self.remove_town(&settlement, &traffic).await {
            return;
        }
        let demand = (self.town_demand_fn)(&settlement);
        self.cx.update_settlement(settlement).await;
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
        let avatar_travel_mode_fn = self.avatar_travel_mode_fn();
        join!(
            self.update_position_traffic_and_refresh_positions(&route_changes),
            self.update_edge_traffic_and_refresh_edges(&route_changes),
            self.update_route_to_ports(&route_changes, &avatar_travel_mode_fn),
        );
    }

    async fn update_position_traffic_and_refresh_positions(&self, route_changes: &[RouteChange]) {
        self.update_all_position_traffic(route_changes).await;
        self.refresh_positions(&route_changes).await
    }

    async fn update_edge_traffic_and_refresh_edges(&self, route_changes: &[RouteChange]) {
        self.update_all_edge_traffic(route_changes).await;
        self.refresh_edges(&route_changes).await
    }

    fn avatar_travel_mode_fn(&self) -> AvatarTravelModeFn {
        AvatarTravelModeFn::new(
            self.cx.parameters().npc_travel.min_navigable_river_width,
            true,
        )
    }

    async fn replenish_sim_queue(&self) {
        let settlements = self
            .cx
            .with_settlements(|settlements| settlements.keys().copied().collect::<Vec<_>>())
            .await;
        self.cx
            .mut_sim_queue(move |sim_queue| {
                if sim_queue.is_empty() {
                    *sim_queue = settlements;
                }
            })
            .await;
    }
}
