use std::collections::HashSet;

use commons::async_trait::async_trait;
use commons::V2;
use futures::FutureExt;

use crate::traits::SendResourceTargets;

#[async_trait]
pub trait RefreshTargets {
    async fn refresh_targets(&self, positions: HashSet<V2<usize>>);
}

#[async_trait]
impl<T> RefreshTargets for T
where
    T: SendResourceTargets,
{
    async fn refresh_targets(&self, positions: HashSet<V2<usize>>) {
        self.send_resource_targets_future(|resource_targets| {
            resource_targets.refresh_targets(positions).boxed()
        })
        .await;
    }
}
