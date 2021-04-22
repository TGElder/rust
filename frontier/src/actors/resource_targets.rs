use crate::resource::{Resource, Resources, RESOURCES};
use crate::traits::{
    GetWorldObjects, InitTargetsWithPlannedRoads, LoadTargetWithPlannedRoads, Target, WithResources,
};
use crate::world::WorldObject;
use commons::grid::Grid;
use commons::{v2, V2};
use std::collections::{HashMap, HashSet};

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
            self.cx.init_targets(resource.name().to_string()).await;
        }
    }

    pub async fn refresh_targets(&self, positions: HashSet<V2<usize>>) {
        let (resources, world_objects) = join!(
            self.get_resources(&positions),
            self.cx.get_world_objects(&positions)
        );

        let targets = get_targets(&positions, &resources, &world_objects);

        self.cx.load_targets(targets).await;
    }

    async fn get_resources(
        &self,
        positions: &HashSet<V2<usize>>,
    ) -> HashMap<V2<usize>, HashSet<Resource>> {
        self.cx
            .with_resources(|resources| {
                positions
                    .iter()
                    .map(|position| (*position, resources.get_cell_unsafe(position).clone()))
                    .collect()
            })
            .await
    }

    async fn all_positions(&self) -> HashSet<V2<usize>> {
        self.cx
            .with_resources(|resources| all_positions(resources))
            .await
    }
}

fn get_targets<'a>(
    positions: &'a HashSet<V2<usize>>,
    resources: &'a HashMap<V2<usize>, HashSet<Resource>>,
    world_objects: &'a HashMap<V2<usize>, WorldObject>,
) -> impl Iterator<Item = Target<'a>> {
    positions.iter().flat_map(move |position| {
        get_targets_at(&position, &resources[position], &world_objects[position])
    })
}

fn get_targets_at<'a>(
    position: &'a V2<usize>,
    resources: &'a HashSet<Resource>,
    world_object: &'a WorldObject,
) -> impl Iterator<Item = Target<'a>> {
    resources.iter().map(move |resource| Target {
        position,
        name: resource.name(),
        target: !blocks(world_object, *resource),
    })
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

pub fn blocks(object: &WorldObject, resource: Resource) -> bool {
    matches!((object, resource),
        (WorldObject::House, Resource::Crops) |
        (WorldObject::House, Resource::Pasture) |
        (WorldObject::House, Resource::Wood) |
        (WorldObject::House, Resource::Stone) |
        (WorldObject::Crop{..}, Resource::Pasture) |
        (WorldObject::Crop{..}, Resource::Wood) |
        (WorldObject::Crop {..}, Resource::Stone) |
        (WorldObject::Pasture, Resource::Wood) |
        (WorldObject::Pasture, Resource::Stone)
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
            resource_targets.cx.get_targets("coal"),
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
            resource_targets.cx.get_targets("crops"),
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
                .get_targets("coal")
                .get_cell_unsafe(&v2(1, 0)),
            true
        );
        assert_eq!(
            *resource_targets
                .cx
                .get_targets("stone")
                .get_cell_unsafe(&v2(1, 0)),
            true
        );
        assert_eq!(
            *resource_targets
                .cx
                .get_targets("crops")
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
                .get_targets("wood")
                .get_cell_unsafe(&v2(1, 0)),
            false
        );
    }
}
