use std::collections::HashSet;

use commons::async_trait::async_trait;
use commons::grid::Grid;
use commons::V2;

use crate::traits::WithWorld;

#[async_trait]
pub trait ExpandPositions {
    async fn expand_positions(&self, positions: &HashSet<V2<usize>>) -> HashSet<V2<usize>>;
}

#[async_trait]
impl<T> ExpandPositions for T
where
    T: WithWorld + Sync,
{
    async fn expand_positions(&self, positions: &HashSet<V2<usize>>) -> HashSet<V2<usize>> {
        self.with_world(|world| {
            positions
                .iter()
                .flat_map(|position| world.expand_position(&position))
                .collect()
        })
        .await
    }
}
