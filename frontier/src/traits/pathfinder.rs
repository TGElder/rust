use commons::async_trait::async_trait;
use std::collections::HashMap;
use std::time::Duration;

use commons::V2;

use crate::pathfinder::traits::PositionsWithin as LegacyPositionsWithin;
use crate::traits::SendPathfinder;

#[async_trait]
pub trait PositionsWithin {
    async fn positions_within(
        &self,
        positions: Vec<V2<usize>>,
        duration: Duration,
    ) -> HashMap<V2<usize>, Duration>;
}

#[async_trait]
impl<T> PositionsWithin for T
where
    T: SendPathfinder + Sync,
{
    async fn positions_within(
        &self,
        positions: Vec<V2<usize>>,
        duration: Duration,
    ) -> HashMap<V2<usize>, Duration> {
        self.send_pathfinder(move |pathfinder| pathfinder.positions_within(&positions, &duration))
            .await
    }
}
