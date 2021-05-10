use commons::async_trait::async_trait;
use futures::future::BoxFuture;

use crate::actors::ResourceTargets;
use crate::traits::{GetWorldObjects, InitTargetsForRouting, LoadTargetForRouting, WithResources};

#[async_trait]
pub trait SendResourceTargets:
    GetWorldObjects + InitTargetsForRouting + LoadTargetForRouting + WithResources + Send + Sync
{
    async fn send_resource_targets_future<F, O>(&self, function: F) -> O
    where
        O: Send + 'static,
        F: FnOnce(&mut ResourceTargets<Self>) -> BoxFuture<O> + Send + 'static;
}
