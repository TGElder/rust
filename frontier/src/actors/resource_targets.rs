use crate::resource::{Resource, Resources, RESOURCES};
use crate::traits::{InitTargetsWithPlannedRoads, LoadTargetWithPlannedRoads, WithResources};
use commons::grid::Grid;
use commons::{v2, V2};
use std::collections::HashSet;

pub struct ResourceTargets<T> {
    cx: T,
}

impl<T> ResourceTargets<T>
where
    T: InitTargetsWithPlannedRoads + LoadTargetWithPlannedRoads + WithResources,
{
    pub fn new(cx: T) -> ResourceTargets<T> {
        ResourceTargets { cx }
    }

    pub async fn init(&mut self) {
        for resource in RESOURCES.iter() {
            self.init_resource(*resource).await;
        }
    }

    async fn init_resource(&mut self, resource: Resource) {
        let targets = self.get_targets(resource).await;
        self.load_targets(target_set(resource), targets).await;
    }

    async fn get_targets(&self, resource: Resource) -> HashSet<V2<usize>> {
        self.cx
            .with_resources(|resources| resource_positions(resources, resource))
            .await
    }

    async fn load_targets(&self, target_set: String, targets: HashSet<V2<usize>>) {
        self.cx.init_targets(target_set.clone()).await;
        for target in targets {
            self.cx.load_target(&target_set, &target, true).await
        }
    }
}

fn resource_positions(resources: &Resources, resource: Resource) -> HashSet<V2<usize>> {
    let mut out = HashSet::new();
    for x in 0..resources.width() {
        for y in 0..resources.height() {
            let position = &v2(x, y);
            if resource_at(&resources, resource, &position) {
                out.insert(*position);
            }
        }
    }
    out
}

fn resource_at(resources: &Resources, resource: Resource, position: &V2<usize>) -> bool {
    resources.get_cell_unsafe(position).contains(&resource)
}

pub fn target_set(resource: Resource) -> String {
    format!("resource-{}", resource.name())
}

#[cfg(test)]
mod tests {
    use super::*;

    use commons::async_trait::async_trait;
    use commons::{v2, M};
    use futures::executor::block_on;
    use std::collections::HashMap;
    use std::sync::Mutex;

    struct Cx {
        targets: Mutex<HashMap<String, M<bool>>>,
        resources: Mutex<Resources>,
    }

    impl Default for Cx {
        fn default() -> Self {
            Cx {
                targets: Mutex::default(),
                resources: Mutex::new(Resources::new(3, 3, Vec::with_capacity(0))),
            }
        }
    }

    #[async_trait]
    impl InitTargetsWithPlannedRoads for Cx {
        async fn init_targets(&self, name: String) {
            self.targets
                .lock()
                .unwrap()
                .insert(name, M::from_element(3, 3, false));
        }
    }

    #[async_trait]
    impl LoadTargetWithPlannedRoads for Cx {
        async fn load_target(&self, name: &str, position: &V2<usize>, target: bool) {
            *self
                .targets
                .lock()
                .unwrap()
                .get_mut(name)
                .unwrap()
                .mut_cell_unsafe(position) = target;
        }
    }

    #[async_trait]
    impl WithResources for Cx {
        async fn with_resources<F, O>(&self, function: F) -> O
        where
            F: FnOnce(&Resources) -> O + Send,
        {
            function(&self.resources.lock().unwrap())
        }

        async fn mut_resources<F, O>(&self, function: F) -> O
        where
            F: FnOnce(&mut Resources) -> O + Send,
        {
            function(&mut self.resources.lock().unwrap())
        }
    }

    #[test]
    #[rustfmt::skip]
    fn test() {

        let cx = Cx::default();
        {
            let mut resources = cx.resources.lock().unwrap();
            *resources.mut_cell_unsafe(&v2(1, 0)) = vec![Resource::Coal];
            *resources.mut_cell_unsafe(&v2(2, 1)) = vec![Resource::Coal];
            *resources.mut_cell_unsafe(&v2(0, 2)) = vec![Resource::Coal, Resource::Whales];
        }

        let mut resource_targets = ResourceTargets::new(cx);
        block_on(resource_targets.init());

        assert_eq!(
            *resource_targets.cx
                .targets
                .lock()
                .unwrap()
                .get("resource-coal")
                .unwrap(),
            M::from_vec(
                3,
                3,
                vec![
                    false, true, false,
                    false, false, true,
                    true, false, false,
                ]
            ),
        );
        assert_eq!(
            *resource_targets.cx
                .targets
                .lock()
                .unwrap()
                .get("resource-crops")
                .unwrap(),
            M::from_element(3, 3, false),
        );
    }
}
