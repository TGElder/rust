use crate::settlement::{Settlement, SettlementClass};
use crate::traits::{Controlled, DrawTown, DrawWorld, ExpandPositions, Micros, WithSettlements};
use commons::async_trait::async_trait;
use commons::V2;

#[async_trait]
pub(in crate::traits) trait InsertSettlement {
    async fn insert_settlement(&self, settlement: Settlement);
}

#[async_trait]
impl<T> InsertSettlement for T
where
    T: WithSettlements + Sync,
{
    async fn insert_settlement(&self, settlement: Settlement) {
        self.mut_settlements(|settlements| settlements.insert(settlement.position, settlement))
            .await;
    }
}

#[async_trait]
pub trait GetSettlement {
    async fn get_settlement(&self, position: &V2<usize>) -> Option<Settlement>;
}

#[async_trait]
impl<T> GetSettlement for T
where
    T: WithSettlements + Sync,
{
    async fn get_settlement(&self, position: &V2<usize>) -> Option<Settlement> {
        self.with_settlements(|settlements| settlements.get(position).cloned())
            .await
    }
}

#[async_trait]
pub trait UpdateSettlement {
    async fn update_settlement(&self, settlement: Settlement);
}

#[async_trait]
impl<T> UpdateSettlement for T
where
    T: Controlled + DrawTown + DrawWorld + ExpandPositions + Micros + WithSettlements + Sync,
{
    async fn update_settlement(&self, settlement: Settlement) {
        let settlement_to_send = settlement.clone();
        let nation_changed = self
            .mut_settlements(|settlements| {
                let new_nation = settlement_to_send.nation.clone();
                settlements
                    .insert(settlement_to_send.position, settlement_to_send)
                    .map(|old| old.nation != new_nation)
                    .unwrap_or(true)
            })
            .await;

        if let SettlementClass::Town = settlement.class {
            if nation_changed {
                let (controlled, micros) =
                    join!(self.controlled(&settlement.position), self.micros(),);
                let tiles = self.expand_positions(&controlled).await;
                self.draw_world_tiles(tiles, micros);
            }

            self.draw_town(settlement);
        }
    }
}

#[async_trait]
pub trait Settlements {
    async fn settlements(&self) -> Vec<Settlement>;
}

#[async_trait]
impl<T> Settlements for T
where
    T: WithSettlements + Sync,
{
    async fn settlements(&self) -> Vec<Settlement> {
        self.with_settlements(|settlements| settlements.values().cloned().collect())
            .await
    }
}
