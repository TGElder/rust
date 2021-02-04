use super::*;

use crate::settlement::{Settlement, SettlementClass::Town};
use crate::traits::{AddCrops, GetSettlement};
use commons::V2;

pub struct CropsBuilder<T> {
    tx: T,
}

#[async_trait]
impl<T> Builder for CropsBuilder<T>
where
    T: AddCrops + GetSettlement + Send + Sync,
{
    fn can_build(&self, build: &Build) -> bool {
        matches!(build, Build::Crops { .. })
    }

    async fn build(&mut self, build: Build) {
        if let Build::Crops { position, rotated } = build {
            self.try_build_crops(position, rotated).await;
        }
    }
}

impl<T> CropsBuilder<T>
where
    T: AddCrops + GetSettlement + Send + Sync,
{
    pub fn new(tx: T) -> CropsBuilder<T> {
        CropsBuilder { tx }
    }

    async fn try_build_crops(&mut self, position: V2<usize>, rotated: bool) {
        if let Some(Settlement { class: Town, .. }) = self.tx.get_settlement(position).await {
            return;
        }
        self.tx.add_crops(position, rotated).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use commons::{v2, Arm};
    use futures::executor::block_on;
    use std::collections::HashMap;

    #[derive(Default)]
    struct Tx {
        crops: Arm<HashMap<V2<usize>, bool>>,
        settlements: HashMap<V2<usize>, Settlement>,
    }

    #[async_trait]
    impl AddCrops for Tx {
        async fn add_crops(&self, position: V2<usize>, rotated: bool) -> bool {
            self.crops.lock().unwrap().insert(position, rotated);
            true
        }
    }

    #[async_trait]
    impl GetSettlement for Tx {
        async fn get_settlement(&self, position: V2<usize>) -> Option<Settlement> {
            self.settlements.get(&position).cloned()
        }
    }

    #[test]
    fn can_build_crops() {
        // Given
        let tx = Tx::default();
        let builder = CropsBuilder::new(tx);

        // When
        let can_build = builder.can_build(&Build::Crops {
            position: v2(1, 2),
            rotated: true,
        });

        // Then
        assert!(can_build);
    }

    #[test]
    fn should_build_crops_if_no_town_on_tile() {
        // Given
        let tx = Tx::default();
        let mut builder = CropsBuilder::new(tx);

        // When
        block_on(builder.build(Build::Crops {
            position: v2(1, 2),
            rotated: true,
        }));

        // Then
        assert_eq!(
            *builder.tx.crops.lock().unwrap(),
            hashmap! {v2(1, 2) => true}
        );
    }

    #[test]
    fn should_not_build_crops_if_town_on_tile() {
        // Given
        let settlement = Settlement {
            position: v2(1, 2),
            class: Town,
            ..Settlement::default()
        };
        let tx = Tx {
            settlements: hashmap! {v2(1, 2) => settlement},
            ..Tx::default()
        };
        let mut builder = CropsBuilder::new(tx);

        // When
        block_on(builder.build(Build::Crops {
            position: v2(1, 2),
            rotated: true,
        }));

        // Then
        assert_eq!(*builder.tx.crops.lock().unwrap(), hashmap! {});
    }
}