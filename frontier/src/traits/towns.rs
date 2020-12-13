use crate::settlement::{Settlement, SettlementClass};
use crate::traits::send::SendSettlements;
use crate::traits::{
    AddController, DrawTown, GetSettlement, InsertSettlement, Micros, RemoveController,
    RemoveWorldObject, SetControlDurations,
};
use commons::async_trait::async_trait;
use commons::V2;

#[async_trait]
pub trait AddTown {
    async fn add_town(&self, town: Settlement) -> bool;
}

#[async_trait]
impl<T> AddTown for T
where
    T: AddController + GetSettlement + InsertSettlement + DrawTown + RemoveWorldObject + Sync,
{
    async fn add_town(&self, town: Settlement) -> bool {
        if town.class != SettlementClass::Town {
            return false;
        }
        if self.get_settlement(town.position).await.is_some() {
            return false;
        }
        let controller = town.position;
        let remove = town.position;
        let to_insert = town.clone();
        join!(self.add_controller(&controller), async {
            self.remove_world_object(remove).await;
            self.insert_settlement(to_insert).await;
            self.draw_town(town);
        });
        true
    }
}

#[async_trait]
pub trait RemoveTown {
    async fn remove_town(&self, position: V2<usize>) -> bool;
}

#[async_trait]
impl<T> RemoveTown for T
where
    T: DrawTown + Micros + RemoveController + SendSettlements + SetControlDurations + Sync,
{
    async fn remove_town(&self, position: V2<usize>) -> bool {
        let settlement = self
            .send_settlements(move |settlements| settlements.remove(&position))
            .await;
        let micros = self.micros().await;
        let settlement = unwrap_or!(settlement, return false);
        if let SettlementClass::Town = settlement.class {
            self.set_control_durations(settlement.position, hashmap! {}, micros)
                .await;
            self.remove_controller(settlement.position).await;
        }
        self.draw_town(settlement);
        true
    }
}