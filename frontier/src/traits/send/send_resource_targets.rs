use commons::async_trait::async_trait;

use crate::actors::ResourceTargets;
use crate::traits::{GetWorldObject, InitTargetsWithPlannedRoads, LoadTargetWithPlannedRoads, WithResources};

#[async_trait]
pub trait SendResourceTargets: GetWorldObject + InitTargetsWithPlannedRoads + LoadTargetWithPlannedRoads + WithResources + Send + Sync {
    async fn send_resource_targets<F, O>(&self, function: F) -> O
    where
        O: Send + 'static,
        F: FnOnce(&mut ResourceTargets<Self>) -> O + Send + 'static;
}
