use super::*;
use crate::settlement::{Settlement, SettlementClass::Homeland};
use crate::traits::{Settlements, UpdateSettlement, VisibleLandPositions};

pub struct UpdateHomelandPopulation<T> {
    tx: T,
}

#[async_trait]
impl<T> Processor for UpdateHomelandPopulation<T>
where
    T: Settlements + UpdateSettlement + VisibleLandPositions + Send + Sync + 'static,
{
    async fn process(&mut self, state: State, instruction: &Instruction) -> State {
        match instruction {
            Instruction::UpdateHomelandPopulation => (),
            _ => return state,
        };
        let visibile_land_positions = self.tx.visible_land_positions().await;
        self.update_homelands(visibile_land_positions as f64).await;
        state
    }
}

impl<T> UpdateHomelandPopulation<T>
where
    T: Settlements + UpdateSettlement + VisibleLandPositions + Send + Sync + 'static,
{
    pub fn new(tx: T) -> UpdateHomelandPopulation<T> {
        UpdateHomelandPopulation { tx }
    }

    async fn update_homelands(&self, total_population: f64) {
        let homelands = self.get_homelands().await;
        let target_population = total_population / homelands.len() as f64;
        for homeland in homelands {
            self.update_homeland(homeland, target_population).await;
        }
    }

    async fn get_homelands(&self) -> Vec<Settlement> {
        self.tx
            .settlements()
            .await
            .into_iter()
            .filter(|settlement| settlement.class == Homeland)
            .collect()
    }

    async fn update_homeland(&self, settlement: Settlement, target_population: f64) {
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
    use commons::{v2, Arm};
    use futures::executor::block_on;
    use std::collections::HashMap;
    use std::sync::Mutex;

    struct Tx {
        settlements: Arm<HashMap<V2<usize>, Settlement>>,
        visible_land_positions: usize,
    }

    #[async_trait]
    impl VisibleLandPositions for Tx {
        async fn visible_land_positions(&self) -> usize {
            self.visible_land_positions
        }
    }

    #[async_trait]
    impl Settlements for Tx {
        async fn settlements(&self) -> Vec<Settlement> {
            self.settlements.lock().unwrap().values().cloned().collect()
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
    fn each_homeland_population_should_be_equal_share_of_visible_land() {
        // Given
        let settlements = Arc::new(Mutex::new(hashmap! {
            v2(0, 1) => Settlement{
                position: v2(0, 1),
                class: Homeland,
                ..Settlement::default()
            },
            v2(0, 2) => Settlement{
                position: v2(0, 2),
                class: Homeland,
                ..Settlement::default()
            },
        }));

        let tx = Tx {
            settlements,
            visible_land_positions: 202,
        };
        let mut processor = UpdateHomelandPopulation::new(tx);

        // When
        block_on(processor.process(State::default(), &Instruction::UpdateHomelandPopulation));

        // Then
        let actual = processor.tx.settlements.lock().unwrap();
        let expected = hashmap! {
            v2(0, 1) => Settlement{
                position: v2(0, 1),
                class: Homeland,
                target_population: 101.0,
                ..Settlement::default()
            },
            v2(0, 2) => Settlement{
                position: v2(0, 2),
                class: Homeland,
                target_population: 101.0,
                ..Settlement::default()
            },
        };
        assert_eq!(*actual, expected);
    }
}