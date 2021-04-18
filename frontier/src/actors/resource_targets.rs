use crate::resource::{Resource, Resources, RESOURCES};
use crate::traits::{
    GetWorldObjects, InitTargetsWithPlannedRoads, LoadTargetWithPlannedRoads, Target, WithResources,
};
use crate::world::WorldObject;
use commons::grid::Grid;
use commons::{v2, V2};
use std::collections::HashSet;
use std::iter::once;

pub struct ResourceTargets<T> {
    cx: T,
}

impl<T> ResourceTargets<T>
where
    T: GetWorldObjects + InitTargetsWithPlannedRoads + LoadTargetWithPlannedRoads + WithResources,
{
    pub fn new(cx: T) -> ResourceTargets<T> {
        ResourceTargets { cx }
    }

    pub async fn init(&self) {
        self.init_targets().await;
        self.refresh_targets(self.all_positions().await).await
    }

    async fn init_targets(&self) {
        for resource in RESOURCES.iter() {
            self.cx.init_targets(target_set(*resource)).await;
        }
    }

    pub async fn refresh_targets(&self, positions: HashSet<V2<usize>>) {
        for position in positions.iter() {
            self.refresh_targets_at(position).await;
        }
    }

    async fn refresh_targets_at(&self, position: &V2<usize>) {
        let resources = self
            .cx
            .with_resources(|resources| resources.get_cell_unsafe(position).clone())
            .await;
        let object = *self
            .cx
            .get_world_objects(&hashset! {*position})
            .await
            .get(position)
            .unwrap();
        for resource in resources {
            self.cx
                .load_targets(once(Target {
                    name: &target_set(resource),
                    position,
                    target: !blocked_by(resource, object),
                }))
                .await;
        }
    }

    async fn all_positions(&self) -> HashSet<V2<usize>> {
        self.cx
            .with_resources(|resources| all_positions(resources))
            .await
    }
}

fn all_positions(resources: &Resources) -> HashSet<V2<usize>> {
    let mut out = HashSet::new();
    for x in 0..resources.width() {
        for y in 0..resources.height() {
            out.insert(v2(x, y));
        }
    }
    out
}

pub fn target_set(resource: Resource) -> String {
    format!("resource-{}", resource.name())
}

pub fn blocked_by(resource: Resource, object: WorldObject) -> bool {
    matches!((resource, object),
        (Resource::Pasture, WorldObject::Crop{..}) |
        (Resource::Wood, WorldObject::Crop{..})
    )
}

#[cfg(test)]
mod tests {
    use crate::traits::Target;

    use super::*;

    use commons::async_trait::async_trait;
    use commons::{v2, M};
    use futures::executor::block_on;
    use std::collections::HashMap;
    use std::sync::Mutex;

    struct Cx {
        resources: Mutex<Resources>,
        targets: Mutex<HashMap<String, M<bool>>>,
        world_object: WorldObject,
    }

    impl Default for Cx {
        fn default() -> Self {
            Cx {
                resources: Mutex::new(Resources::new(3, 3, HashSet::with_capacity(0))),
                targets: Mutex::default(),
                world_object: WorldObject::None,
            }
        }
    }

    impl Cx {
        fn get_targets(&self, target_set: &str) -> M<bool> {
            self.targets
                .lock()
                .unwrap()
                .get(target_set)
                .unwrap()
                .clone()
        }
    }

    #[async_trait]
    impl GetWorldObjects for Cx {
        async fn get_world_objects(
            &self,
            positions: &HashSet<V2<usize>>,
        ) -> HashMap<V2<usize>, WorldObject> {
            positions
                .iter()
                .map(|position| (*position, self.world_object))
                .collect()
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
        async fn load_targets<'a, I>(&self, targets: I)
        where
            I: Iterator<Item = Target<'a>> + Send,
        {
            for Target {
                name,
                position,
                target,
            } in targets
            {
                *self
                    .targets
                    .lock()
                    .unwrap()
                    .get_mut(name)
                    .unwrap()
                    .mut_cell_unsafe(position) = target;
            }
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
    fn test_init() {
        // Given
        let cx = Cx::default();
        {
            let mut resources = cx.resources.lock().unwrap();
            *resources.mut_cell_unsafe(&v2(1, 0)) = hashset!{Resource::Coal};
            *resources.mut_cell_unsafe(&v2(2, 1)) = hashset!{Resource::Coal};
            *resources.mut_cell_unsafe(&v2(0, 2)) = hashset!{Resource::Coal, Resource::Whales};
        }

        let resource_targets = ResourceTargets::new(cx);

        // When
        block_on(resource_targets.init());

        // Then
        assert_eq!(
            resource_targets.cx.get_targets("resource-coal"),
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
            resource_targets.cx.get_targets("resource-crops"),
            M::from_element(3, 3, false),
        );
    }

    #[test]
    fn test_refresh_targets_at() {
        // Given
        let resource_targets = ResourceTargets::new(Cx::default());
        block_on(resource_targets.init());
        {
            let mut resources = resource_targets.cx.resources.lock().unwrap();
            *resources.mut_cell_unsafe(&v2(1, 0)) = hashset! {Resource::Coal, Resource::Stone};
        }

        // When
        block_on(resource_targets.refresh_targets(hashset! {v2(1, 0)}));

        // Then
        assert_eq!(
            *resource_targets
                .cx
                .get_targets("resource-coal")
                .get_cell_unsafe(&v2(1, 0)),
            true
        );
        assert_eq!(
            *resource_targets
                .cx
                .get_targets("resource-stone")
                .get_cell_unsafe(&v2(1, 0)),
            true
        );
        assert_eq!(
            *resource_targets
                .cx
                .get_targets("resource-crops")
                .get_cell_unsafe(&v2(1, 0)),
            false
        );
    }

    #[test]
    fn test_refresh_targets_at_blocked_by() {
        // Given
        let cx = Cx {
            world_object: WorldObject::Crop { rotated: true },
            ..Cx::default()
        };
        let resource_targets = ResourceTargets::new(cx);
        block_on(resource_targets.init());
        {
            let mut resources = resource_targets.cx.resources.lock().unwrap();
            *resources.mut_cell_unsafe(&v2(1, 0)) = hashset! {Resource::Wood};
        }

        // When
        block_on(resource_targets.refresh_targets(hashset! {v2(1, 0)}));

        // Then
        assert_eq!(
            *resource_targets
                .cx
                .get_targets("resource-wood")
                .get_cell_unsafe(&v2(1, 0)),
            false
        );
    }
}
