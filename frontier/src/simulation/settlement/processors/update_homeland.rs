use crate::settlement::Settlement;
use crate::traits::{UpdateSettlement, VisibleLandPositions};

pub struct UpdateHomeland<T> {
    tx: T,
    homeland_count: usize,
}

impl<T> UpdateHomeland<T>
where
    T: UpdateSettlement + VisibleLandPositions,
{
    pub fn new(tx: T, homeland_count: usize) -> UpdateHomeland<T> {
        UpdateHomeland { tx, homeland_count }
    }

    pub async fn update_homeland(&self, settlement: &Settlement) {
        let visible_land_positions = self.tx.visible_land_positions().await;
        let target_population = visible_land_positions as f64 / self.homeland_count as f64;
        self.update_population(settlement.clone(), target_population)
            .await;
    }

    async fn update_population(&self, settlement: Settlement, target_population: f64) {
        let settlement = Settlement {
            target_population,
            ..settlement
        };
        self.tx.update_settlement(settlement).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settlement::SettlementClass::Homeland;
    use commons::async_trait::async_trait;
    use commons::{v2, V2};
    use futures::executor::block_on;
    use std::collections::HashMap;
    use std::sync::Mutex;

    struct Tx {
        settlements: Mutex<HashMap<V2<usize>, Settlement>>,
        visible_land_positions: usize,
    }

    #[async_trait]
    impl VisibleLandPositions for Tx {
        async fn visible_land_positions(&self) -> usize {
            self.visible_land_positions
        }
    }

    #[async_trait]
    impl UpdateSettlement for Tx {
        async fn update_settlement(&self, settlement: Settlement) {
            self.settlements
                .lock()
                .unwrap()
                .insert(settlement.position, settlement);
        }
    }

    #[test]
    fn target_population_should_be_equal_share_of_visible_land() {
        // Given
        let settlement = Settlement {
            position: v2(0, 1),
            class: Homeland,
            ..Settlement::default()
        };
        let settlements = Mutex::new(hashmap! {
            v2(0, 1) => settlement.clone()
        });

        let tx = Tx {
            settlements,
            visible_land_positions: 202,
        };
        let update_homeland = UpdateHomeland::new(tx, 2);

        // When
        block_on(update_homeland.update_homeland(&settlement));

        // Then
        let actual = update_homeland.tx.settlements.lock().unwrap();
        let expected = hashmap! {
            v2(0, 1) => Settlement{
                position: v2(0, 1),
                class: Homeland,
                target_population: 101.0,
                ..Settlement::default()
            }
        };
        assert_eq!(*actual, expected);
    }
}
