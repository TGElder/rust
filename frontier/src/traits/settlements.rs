use crate::settlement::Settlement;
use crate::traits::send::SendSettlements;
use commons::async_trait::async_trait;
use commons::V2;

#[async_trait]
pub trait GetSettlement {
    async fn get_settlement(&self, position: V2<usize>) -> Option<Settlement>;
}

#[async_trait]
impl<T> GetSettlement for T
where
    T: SendSettlements + Sync,
{
    async fn get_settlement(&self, position: V2<usize>) -> Option<Settlement> {
        self.send_settlements(move |settlements| settlements.get(&position).cloned())
            .await
    }
}

#[async_trait]
pub trait Settlements {
    async fn settlements(&self) -> Vec<Settlement>;
}

#[async_trait]
impl<T> Settlements for T
where
    T: SendSettlements + Sync,
{
    async fn settlements(&self) -> Vec<Settlement> {
        self.send_settlements(move |settlements| settlements.values().cloned().collect())
            .await
    }
}
