use crate::settlement::{Settlement, SettlementClass};
use crate::traits::{
    AddController, DrawTown, GetSettlement, InsertSettlement, Micros, RemoveController,
    RemoveWorldObjects, SetControlDurations, Visibility, WithSettlements, WithWorld,
};
use commons::async_trait::async_trait;
use commons::grid::Grid;
use commons::V2;

#[async_trait]
pub trait AddTown {
    async fn add_town(&self, town: Settlement) -> bool;
}

#[async_trait]
impl<T> AddTown for T
where
    T: AddController
        + GetSettlement
        + DrawTown
        + RemoveWorldObjects
        + Visibility
        + WithSettlements
        + WithWorld
        + Sync,
{
    async fn add_town(&self, town: Settlement) -> bool {
        if town.class != SettlementClass::Town {
            return false;
        }
        if self.get_settlement(&town.position).await.is_some() {
            return false;
        }
        let controller = town.position;
        let remove = town.position;
        let to_insert = town.clone();

        join!(
            self.add_controller(controller),
            check_visibility_from_town(self, town.position),
            async {
                self.remove_world_objects(&hashset! {remove}).await;
                self.insert_settlement(to_insert).await;
                self.draw_town(town);
            }
        );
        true
    }
}

async fn check_visibility_from_town<T>(cx: &T, position: V2<usize>)
where
    T: Visibility + WithWorld,
{
    let visited = cx
        .with_world(|world| world.get_corners_in_bounds(&position))
        .await;
    cx.check_visibility_and_reveal(&visited.into_iter().collect())
        .await;
}

#[async_trait]
pub trait RemoveTown {
    async fn remove_town(&self, position: &V2<usize>) -> bool;
}

#[async_trait]
impl<T> RemoveTown for T
where
    T: DrawTown + Micros + RemoveController + SetControlDurations + WithSettlements + Sync,
{
    async fn remove_town(&self, position: &V2<usize>) -> bool {
        let settlement = self
            .mut_settlements(|settlements| settlements.remove(position))
            .await;
        let micros = self.micros().await;
        let settlement = unwrap_or!(settlement, return false);
        if let SettlementClass::Town = settlement.class {
            self.set_control_durations(settlement.position, &hashmap! {}, &micros)
                .await;
            self.remove_controller(&settlement.position).await;
        }
        self.draw_town(settlement);
        true
    }
}
