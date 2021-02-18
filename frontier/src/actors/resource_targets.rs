use crate::resource::{Resource, RESOURCES};
use crate::traits::{InitTargetsWithPlannedRoads, LoadTargetWithPlannedRoads, WithWorld};
use crate::world::World;
use commons::grid::Grid;
use commons::{v2, V2};
use std::collections::HashSet;

pub struct ResourceTargets<T> {
    cx: T,
}

impl<T> ResourceTargets<T>
where
    T: InitTargetsWithPlannedRoads + LoadTargetWithPlannedRoads + WithWorld,
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
            .with_world(|world| resource_positions(world, resource))
            .await
    }

    async fn load_targets(&self, target_set: String, targets: HashSet<V2<usize>>) {
        self.cx.init_targets(target_set.clone()).await;
        for target in targets {
            self.cx.load_target(&target_set, &target, true).await
        }
    }
}

fn resource_positions(world: &World, resource: Resource) -> HashSet<V2<usize>> {
    let mut out = HashSet::new();
    for x in 0..world.width() {
        for y in 0..world.height() {
            let position = &v2(x, y);
            if resource_at(&world, resource, &position) {
                out.insert(*position);
            }
        }
    }
    out
}

fn resource_at(world: &World, resource: Resource, position: &V2<usize>) -> bool {
    matches!(world.get_cell(position), Some(cell) if cell.resource == resource)
}

pub fn target_set(resource: Resource) -> String {
    format!("resource-{}", resource.name())
}

#[cfg(test)]
mod tests {
    use super::*;

    use commons::async_trait::async_trait;
    use commons::{v2, Arm, M};
    use futures::executor::block_on;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    struct Cx {
        targets: Arm<HashMap<String, M<bool>>>,
        world: Arm<World>,
    }

    impl Default for Cx {
        fn default() -> Self {
            Cx {
                targets: Arm::default(),
                world: Arc::new(Mutex::new(World::new(M::zeros(3, 3), 0.5))),
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
    impl WithWorld for Cx {
        async fn with_world<F, O>(&self, function: F) -> O
        where
            F: FnOnce(&World) -> O + Send,
        {
            function(&self.world.lock().unwrap())
        }

        async fn mut_world<F, O>(&self, function: F) -> O
        where
            F: FnOnce(&mut World) -> O + Send,
        {
            function(&mut self.world.lock().unwrap())
        }
    }

    #[test]
    #[rustfmt::skip]
    fn test() {

        let cx = Cx::default();
        {
            let mut world = cx.world.lock().unwrap();
            world.mut_cell_unsafe(&v2(1, 0)).resource = Resource::Coal;
            world.mut_cell_unsafe(&v2(2, 1)).resource = Resource::Coal;
            world.mut_cell_unsafe(&v2(0, 2)).resource = Resource::Coal;
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
