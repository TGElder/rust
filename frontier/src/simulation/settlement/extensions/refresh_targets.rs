use crate::actors::target_set;
use crate::resource::Resource;
use crate::route::RouteKey;
use crate::simulation::settlement::model::RouteChange;
use crate::simulation::settlement::SettlementSimulation;
use crate::traits::{LoadTargetWithPlannedRoads, RefreshTargets, WithResources, WithTraffic};
use commons::log::debug;

impl<T> SettlementSimulation<T>
where
    T: RefreshTargets,
{
    pub async fn refresh_targets(&self, route_changes: &[RouteChange]) {
        for route_change in route_changes {
            self.refresh_targets_for_change(route_change).await;
        }
    }

    async fn refresh_targets_for_change(&self, route_change: &RouteChange) {
        match route_change {
            RouteChange::New { key, .. } => self.refresh_targets_for_new(key).await,
            RouteChange::Removed { key, .. } => self.refresh_targets_for_removed(key).await,
            _ => (),
        }
    }

    async fn refresh_targets_for_new(&self, key: &RouteKey) {
        if key.resource != Resource::Crops {
            return;
        }
        let resources = unwrap_or!(
            self.cx
                .with_resources(|resources| resources.get(&key.destination).ok().cloned())
                .await,
            return
        );
        for resource in resources {
            if resource == Resource::Wood {
                debug!("Removing {} at {}", resource.name(), key.destination);
                self.cx
                    .load_target(&target_set(resource), &key.destination, false)
                    .await;
            }
        }
    }

    async fn refresh_targets_for_removed(&self, key: &RouteKey) {
        if key.resource != Resource::Crops {
            return;
        }
        let traffic = unwrap_or!(
            self.cx
                .with_traffic(|traffic| traffic.get(&key.destination).ok().cloned())
                .await,
            return
        );
        if traffic.iter().any(|key| key.resource == Resource::Crops) {
            return;
        }
        let resources = unwrap_or!(
            self.cx
                .with_resources(|resources| resources.get(&key.destination).ok().cloned())
                .await,
            return
        );
        for resource in resources {
            if resource == Resource::Wood {
                debug!("Restoring {} at {}", resource.name(), key.destination);
                self.cx
                    .load_target(&target_set(resource), &key.destination, true)
                    .await;
            }
        }
    }
}