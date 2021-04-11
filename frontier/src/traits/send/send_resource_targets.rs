use commons::async_trait::async_trait;
use futures::future::BoxFuture;

use crate::actors::ResourceTargets;
use crate::traits::{
    GetWorldObject, InitTargetsWithPlannedRoads, LoadTargetWithPlannedRoads, WithResources,
};

#[async_trait]
pub trait SendResourceTargets:
    GetWorldObject
    + InitTargetsWithPlannedRoads
    + LoadTargetWithPlannedRoads
    + WithResources
    + Send
    + Sync
{
    async fn send_resource_targets_future<F, O>(&self, function: F) -> O
    where
        O: Send + 'static,
        F: FnOnce(&mut ResourceTargets<Self>) -> BoxFuture<O> + Send + 'static;
}
