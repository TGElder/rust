use std::collections::HashSet;

use commons::async_trait::async_trait;
use commons::{Grid, V2};

use crate::traits::SendWorld;

#[async_trait]
pub trait ExpandPositions {
    async fn expand_positions(&self, positions: HashSet<V2<usize>>) -> HashSet<V2<usize>>;
}

#[async_trait]
impl<T> ExpandPositions for T
where
    T: SendWorld + Sync,
{
    async fn expand_positions(&self, positions: HashSet<V2<usize>>) -> HashSet<V2<usize>> {
        self.send_world(move |world| {
            positions
                .iter()
                .flat_map(|position| world.expand_position(&position))
                .collect()
        })
        .await
    }
}
