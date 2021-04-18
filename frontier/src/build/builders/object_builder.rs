use super::*;

use crate::settlement::{Settlement, SettlementClass::Town};
use crate::traits::{GetSettlement, RefreshTargets, SetWorldObjects};
use crate::world::WorldObject;
use commons::log::debug;
use commons::V2;

pub struct ObjectBuilder<T> {
    cx: T,
}

#[async_trait]
impl<T> Builder for ObjectBuilder<T>
where
    T: SetWorldObjects + GetSettlement + RefreshTargets + Send + Sync,
{
    fn can_build(&self, build: &Build) -> bool {
        matches!(build, Build::Object { .. })
    }

    async fn build(&mut self, build: Vec<Build>) {
        let start = std::time::Instant::now();
        let count = build.len();
        for build in build {
            self.try_build(build).await;
        }
        debug!(
            "Took {}ms to build {} objects",
            start.elapsed().as_millis(),
            count
        );
    }
}

impl<T> ObjectBuilder<T>
where
    T: GetSettlement + RefreshTargets + SetWorldObjects + Send + Sync,
{
    pub fn new(cx: T) -> ObjectBuilder<T> {
        ObjectBuilder { cx }
    }

    async fn try_build(&self, build: Build) {
        if let Build::Object { position, object } = build {
            self.try_build_object(&position, object).await;
        }
    }

    async fn try_build_object(&self, position: &V2<usize>, object: WorldObject) {
        if let Some(Settlement { class: Town, .. }) = self.cx.get_settlement(position).await {
            return;
        }
        self.cx
            .set_world_objects(&hashmap! {*position => object})
            .await;
        self.cx.refresh_targets(hashset! {*position}).await;
    }
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
    impl GetSettlement for Cx {
        async fn get_settlement(&self, position: &V2<usize>) -> Option<Settlement> {
            self.settlements.get(position).cloned()
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
            class: Town,
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
