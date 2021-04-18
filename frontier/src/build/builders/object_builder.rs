use std::collections::{HashMap, HashSet};

use super::*;

use crate::settlement::Settlement;
use crate::traits::{RefreshTargets, SetWorldObjects, Settlements};
use crate::world::WorldObject;
use commons::V2;

pub struct ObjectBuilder<T> {
    cx: T,
}

#[async_trait]
impl<T> Builder for ObjectBuilder<T>
where
    T: RefreshTargets + Settlements + SetWorldObjects + Send + Sync,
{
    fn can_build(&self, build: &Build) -> bool {
        matches!(build, Build::Object { .. })
    }

    async fn build(&mut self, build: Vec<Build>) {
        let objects = self.get_objects_to_build(build).await;
        self.cx.set_world_objects(&objects).await;

        let positions = objects.into_iter().map(|(position, _)| position).collect();
        self.cx.refresh_targets(positions).await;
    }
}

impl<T> ObjectBuilder<T>
where
    T: RefreshTargets + Settlements + SetWorldObjects + Send + Sync,
{
    pub fn new(cx: T) -> ObjectBuilder<T> {
        ObjectBuilder { cx }
    }

    async fn get_objects_to_build(&self, build: Vec<Build>) -> HashMap<V2<usize>, WorldObject> {
        let settlements = self.get_settlement_positions().await;
        get_objects_to_build(build)
            .into_iter()
            .filter(|(position, _)| !settlements.contains(position))
            .collect::<HashMap<_, _>>()
    }

    async fn get_settlement_positions(&self) -> HashSet<V2<usize>> {
        self.cx
            .settlements()
            .await
            .into_iter()
            .map(|Settlement { position, .. }| position)
            .collect()
    }
}

fn get_objects_to_build(build: Vec<Build>) -> HashMap<V2<usize>, WorldObject> {
    build
        .into_iter()
        .flat_map(try_get_object_to_build)
        .collect()
}

fn try_get_object_to_build(build: Build) -> Option<(V2<usize>, WorldObject)> {
    if let Build::Object { position, object } = build {
        return Some((position, object));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    use commons::v2;
    use futures::executor::block_on;
    use std::collections::{HashMap, HashSet};
    use std::sync::Mutex;

    #[derive(Default)]
    struct Cx {
        refreshed_targets: Mutex<HashSet<V2<usize>>>,
        settlements: HashMap<V2<usize>, Settlement>,
        world_objects: Mutex<HashMap<V2<usize>, WorldObject>>,
    }

    #[async_trait]
    impl SetWorldObjects for Cx {
        async fn set_world_objects(&self, objects: &HashMap<V2<usize>, WorldObject>) {
            self.world_objects.lock().unwrap().extend(objects);
        }
    }

    #[async_trait]
    impl RefreshTargets for Cx {
        async fn refresh_targets(&self, positions: HashSet<V2<usize>>) {
            self.refreshed_targets.lock().unwrap().extend(positions);
        }
    }

    #[async_trait]
    impl Settlements for Cx {
        async fn settlements(&self) -> Vec<Settlement> {
            self.settlements.values().cloned().collect()
        }
    }

    #[test]
    fn can_build_object() {
        // Given
        let cx = Cx::default();
        let builder = ObjectBuilder::new(cx);

        // When
        let can_build = builder.can_build(&Build::Object {
            position: v2(1, 2),
            object: WorldObject::Crop { rotated: true },
        });

        // Then
        assert!(can_build);
    }

    #[test]
    fn should_build_object_if_no_town_on_tile() {
        // Given
        let cx = Cx::default();
        let object = WorldObject::Crop { rotated: true };
        let mut builder = ObjectBuilder::new(cx);

        // When
        block_on(builder.build(vec![Build::Object {
            position: v2(1, 2),
            object,
        }]));

        // Then
        assert_eq!(
            *builder.cx.world_objects.lock().unwrap(),
            hashmap! {v2(1, 2) => object}
        );
        assert_eq!(
            *builder.cx.refreshed_targets.lock().unwrap(),
            hashset! { v2(1, 2) },
        );
    }

    #[test]
    fn should_not_build_object_if_town_on_tile() {
        // Given
        let settlement = Settlement {
            position: v2(1, 2),
            ..Settlement::default()
        };
        let cx = Cx {
            settlements: hashmap! {v2(1, 2) => settlement},
            ..Cx::default()
        };
        let mut builder = ObjectBuilder::new(cx);

        // When
        block_on(builder.build(vec![Build::Object {
            position: v2(1, 2),
            object: WorldObject::Crop { rotated: true },
        }]));

        // Then
        assert!(builder.cx.world_objects.lock().unwrap().is_empty());
        assert!(builder.cx.refreshed_targets.lock().unwrap().is_empty());
    }

    #[test]
    fn should_build_all_objects() {
        // Given
        let cx = Cx::default();
        let crops = WorldObject::Crop { rotated: true };
        let mut builder = ObjectBuilder::new(cx);

        // When
        block_on(builder.build(vec![
            Build::Object {
                position: v2(1, 2),
                object: crops,
            },
            Build::Object {
                position: v2(3, 4),
                object: WorldObject::None,
            },
        ]));

        // Then
        assert_eq!(
            *builder.cx.world_objects.lock().unwrap(),
            hashmap! {
                v2(1, 2) => crops,
                v2(3, 4) => WorldObject::None,
            }
        );
    }
}
