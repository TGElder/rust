use std::collections::{HashMap, HashSet};

use super::*;

use crate::resource::Mine;
use crate::settlement::Settlement;
use crate::traits::{RefreshTargets, SetWorldObjects, Settlements};
use crate::world::WorldObject;
use commons::rand::rngs::SmallRng;
use commons::rand::SeedableRng;
use commons::V2;

pub struct MineBuilder<T> {
    cx: T,
    rng: SmallRng,
}

#[async_trait]
impl<T> Builder for MineBuilder<T>
where
    T: RefreshTargets + Settlements + SetWorldObjects + Send + Sync,
{
    fn can_build(&self, build: &Build) -> bool {
        matches!(build, Build::Mine { .. })
    }

    async fn build(&mut self, build: Vec<Build>) {
        let mines = get_mines_to_build(build);
        let mines = self.filter_out_positions_with_settlements(mines).await;

        let objects = self.get_objects_to_build(mines);
        self.cx.set_world_objects(&objects).await;

        let positions = objects.into_iter().map(|(position, _)| position).collect();
        self.cx.refresh_targets(positions).await;
    }
}

impl<T> MineBuilder<T>
where
    T: RefreshTargets + Settlements + SetWorldObjects + Send + Sync,
{
    pub fn new(cx: T, seed: u64) -> MineBuilder<T> {
        MineBuilder {
            cx,
            rng: SeedableRng::seed_from_u64(seed),
        }
    }

    async fn filter_out_positions_with_settlements(
        &self,
        objects: HashMap<V2<usize>, Mine>,
    ) -> HashMap<V2<usize>, Mine> {
        let settlements = self.get_settlement_positions().await;
        objects
            .into_iter()
            .filter(|(position, _)| !settlements.contains(position))
            .collect()
    }

    async fn get_settlement_positions(&self) -> HashSet<V2<usize>> {
        self.cx
            .settlements()
            .await
            .into_iter()
            .map(|Settlement { position, .. }| position)
            .collect()
    }

    fn get_objects_to_build(
        &mut self,
        mines: HashMap<V2<usize>, Mine>,
    ) -> HashMap<V2<usize>, WorldObject> {
        mines
            .into_iter()
            .map(|(position, mine)| (position, mine.get_world_object(&mut self.rng)))
            .collect()
    }
}

fn get_mines_to_build(build: Vec<Build>) -> HashMap<V2<usize>, Mine> {
    build.into_iter().flat_map(try_get_mine_to_build).collect()
}

fn try_get_mine_to_build(build: Build) -> Option<(V2<usize>, Mine)> {
    if let Build::Mine { position, mine } = build {
        return Some((position, mine));
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
        let builder = MineBuilder::new(cx, 0);

        // When
        let can_build = builder.can_build(&Build::Mine {
            position: v2(1, 2),
            mine: Mine::Pasture,
        });

        // Then
        assert!(can_build);
    }

    #[test]
    fn should_build_object_if_no_town_on_tile() {
        // Given
        let cx = Cx::default();
        let mine = Mine::Pasture;
        let mut builder = MineBuilder::new(cx, 0);

        // When
        block_on(builder.build(vec![Build::Mine {
            position: v2(1, 2),
            mine,
        }]));

        // Then
        assert_eq!(
            *builder.cx.world_objects.lock().unwrap(),
            hashmap! {v2(1, 2) => WorldObject::Pasture}
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
        let mut builder = MineBuilder::new(cx, 0);

        // When
        block_on(builder.build(vec![Build::Mine {
            position: v2(1, 2),
            mine: Mine::Pasture,
        }]));

        // Then
        assert!(builder.cx.world_objects.lock().unwrap().is_empty());
        assert!(builder.cx.refreshed_targets.lock().unwrap().is_empty());
    }

    #[test]
    fn should_build_all_objects() {
        // Given
        let cx = Cx::default();
        let mut builder = MineBuilder::new(cx, 0);

        // When
        block_on(builder.build(vec![
            Build::Mine {
                position: v2(1, 2),
                mine: Mine::Pasture,
            },
            Build::Mine {
                position: v2(3, 4),
                mine: Mine::None,
            },
        ]));

        // Then
        assert_eq!(
            *builder.cx.world_objects.lock().unwrap(),
            hashmap! {
                v2(1, 2) => WorldObject::Pasture,
                v2(3, 4) => WorldObject::None,
            }
        );
    }
}
