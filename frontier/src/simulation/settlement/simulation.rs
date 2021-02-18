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
    ClosestTargetsWithPlannedRoads, Controlled, CostOfPathWithoutPlannedRoads, GetSettlement,
    InBoundsWithPlannedRoads, Micros, RefreshEdges, RefreshPositions, RemoveTown,
    UpdateSettlement as UpdateSettlementTrait, UpdateTerritory, VisibleLandPositions,
    WithEdgeTraffic, WithRouteToPorts, WithRoutes, WithSettlements, WithSimQueue, WithTraffic,
    WithWorld,
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
        + CostOfPathWithoutPlannedRoads
        + GetSettlement
        + HasParameters
        + InBoundsWithPlannedRoads
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
        + CostOfPathWithoutPlannedRoads
        + GetSettlement
        + HasParameters
        + InBoundsWithPlannedRoads
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
{
    async fn update_settlement_at(&self, position: &V2<usize>) {
        let settlement = unwrap_or!(self.tx.get_settlement(position).await, return);
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
            self.update_edge_traffic(&route_changes),
            self.update_position_traffic(&route_changes),
            self.update_route_to_ports(&route_changes, &avatar_travel_mode_fn),
        );
    }

    fn avatar_travel_mode_fn(&self) -> AvatarTravelModeFn {
        AvatarTravelModeFn::new(self.tx.parameters().avatar_travel.min_navigable_river_width)
    }

    async fn replenish_sim_queue(&self) {
        let settlements = self
            .tx
            .with_settlements(|settlements| settlements.keys().copied().collect::<Vec<_>>())
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
