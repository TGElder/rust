use super::*;
use crate::settlement::{Settlement, SettlementClass};
use crate::traits::{GetSettlement, Micros, UpdateSettlement};

pub struct UpdateCurrentPopulation<T> {
    tx: T,
    max_abs_population_change: fn(&SettlementClass) -> f64,
}

#[async_trait]
impl<T> Processor for UpdateCurrentPopulation<T>
where
    T: GetSettlement + Micros + UpdateSettlement + Send + Sync + 'static,
{
    async fn process(&mut self, mut state: State, instruction: &Instruction) -> State {
        let position = match instruction {
            Instruction::UpdateCurrentPopulation(position) => *position,
            _ => return state,
        };

        if let Some(settlement) = self.try_update_settlement(position).await {
            state.instructions.push(Instruction::GetDemand(settlement))
        }

        state
    }
}

impl<T> UpdateCurrentPopulation<T>
where
    T: GetSettlement + Micros + UpdateSettlement + Send,
{
    pub fn new(
        tx: T,
        max_abs_population_change: fn(&SettlementClass) -> f64,
    ) -> UpdateCurrentPopulation<T> {
        UpdateCurrentPopulation {
            tx,
            max_abs_population_change,
        }
    }

    async fn try_update_settlement(&mut self, position: V2<usize>) -> Option<Settlement> {
        let max_abs_population_change = self.max_abs_population_change;
        let settlement = self.tx.get_settlement(position).await?;
        let game_micros = self.tx.micros().await;

        if settlement.last_population_update_micros >= game_micros {
            return Some(settlement.clone());
        }

        let change = clamp_population_change(
            get_population_change(&settlement, &game_micros),
            max_abs_population_change(&settlement.class),
        );
        let current_population = settlement.current_population + change;

        let new_settlement = Settlement {
            current_population,
            last_population_update_micros: game_micros,
            name: settlement.name.clone(),
            nation: settlement.nation.clone(),
            ..settlement
        };
        self.tx.update_settlement(new_settlement.clone()).await;
        Some(new_settlement)
    }
}

fn get_population_change(settlement: &Settlement, game_micros: &u128) -> f64 {
    let half_life = settlement.gap_half_life.as_micros() as f64;
    if half_life == 0.0 {
        settlement.target_population - settlement.current_population
    } else {
        let last_update_micros = settlement.last_population_update_micros;
        let elapsed = (game_micros - last_update_micros) as f64;
        let exponent = elapsed / half_life;
        let gap_decay = 1.0 - 0.5f64.powf(exponent);
        (settlement.target_population - settlement.current_population) * gap_decay
    }
}

fn clamp_population_change(population_change: f64, max_abs_change: f64) -> f64 {
    population_change.max(-max_abs_change).min(max_abs_change)
}

pub fn max_abs_population_change(settlement_class: &SettlementClass) -> f64 {
    match settlement_class {
        SettlementClass::Town => 16.0,
        SettlementClass::Homeland => 128.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use commons::almost::Almost;
    use commons::{v2, Arm};
    use futures::executor::block_on;
    use std::collections::HashMap;
    use std::sync::Mutex;
    use std::time::Duration;

    struct Tx {
        micros: u128,
        settlements: Arm<HashMap<V2<usize>, Settlement>>,
    }

    #[async_trait]
    impl GetSettlement for Tx {
        async fn get_settlement(&self, position: V2<usize>) -> Option<Settlement> {
            self.settlements.lock().unwrap().get(&position).cloned()
        }
    }

    #[async_trait]
    impl Micros for Tx {
        async fn micros(&self) -> u128 {
            self.micros
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

    fn max_abs_population_change(_: &SettlementClass) -> f64 {
        100.0
    }

    #[test]
    fn should_move_current_population_towards_target_population_when_target_more() {
        // Given
        let settlement = Settlement {
            position: v2(1, 2),
            current_population: 1.0,
            target_population: 100.0,
            gap_half_life: Duration::from_micros(10),
            last_population_update_micros: 11,
            ..Settlement::default()
        };
        let settlements = Arc::new(Mutex::new(hashmap! {
            v2(1, 2) => settlement,
        }));

        let tx = Tx {
            micros: 33,
            settlements,
        };
        let mut processor = UpdateCurrentPopulation::new(tx, max_abs_population_change);

        // When
        let state = block_on(processor.process(
            State::default(),
            &Instruction::UpdateCurrentPopulation(v2(1, 2)),
        ));

        // Then
        let settlements = processor.tx.settlements.lock().unwrap();
        let settlement = settlements.get(&v2(1, 2)).unwrap();

        assert!(settlement.current_population.almost(&78.45387355842092));
        assert_eq!(settlement.last_population_update_micros, 33);
        assert_eq!(
            state.instructions,
            vec![Instruction::GetDemand(settlement.clone())]
        );
    }

    #[test]
    fn should_move_current_population_towards_target_population_when_target_less() {
        // Given
        let settlement = Settlement {
            position: v2(1, 2),
            current_population: 100.0,
            target_population: 1.0,
            gap_half_life: Duration::from_micros(10),
            last_population_update_micros: 11,
            ..Settlement::default()
        };
        let settlements = Arc::new(Mutex::new(hashmap! {
            v2(1, 2) => settlement,
        }));

        let tx = Tx {
            micros: 33,
            settlements,
        };
        let mut processor = UpdateCurrentPopulation::new(tx, max_abs_population_change);

        // When
        let state = block_on(processor.process(
            State::default(),
            &Instruction::UpdateCurrentPopulation(v2(1, 2)),
        ));

        // Then
        let settlements = processor.tx.settlements.lock().unwrap();
        let settlement = settlements.get(&v2(1, 2)).unwrap();

        assert!(settlement.current_population.almost(&22.54612644157907));
        assert_eq!(settlement.last_population_update_micros, 33);
        assert_eq!(
            state.instructions,
            vec![Instruction::GetDemand(settlement.clone())]
        );
    }

    #[test]
    fn should_set_current_population_to_target_population_if_half_life_zero() {
        // Given
        let settlement = Settlement {
            position: v2(1, 2),
            current_population: 100.0,
            target_population: 1.0,
            gap_half_life: Duration::from_micros(0),
            last_population_update_micros: 11,
            ..Settlement::default()
        };
        let settlements = Arc::new(Mutex::new(hashmap! {
            v2(1, 2) => settlement,
        }));

        let tx = Tx {
            micros: 33,
            settlements,
        };
        let mut processor = UpdateCurrentPopulation::new(tx, max_abs_population_change);

        // When
        let state = block_on(processor.process(
            State::default(),
            &Instruction::UpdateCurrentPopulation(v2(1, 2)),
        ));

        // Then
        let settlements = processor.tx.settlements.lock().unwrap();
        let settlement = settlements.get(&v2(1, 2)).unwrap();

        assert!(settlement
            .current_population
            .almost(&settlement.target_population));
        assert_eq!(settlement.last_population_update_micros, 33);
        assert_eq!(
            state.instructions,
            vec![Instruction::GetDemand(settlement.clone())]
        );
    }

    #[test]
    fn should_do_nothing_if_no_settlement() {
        // Given

        let tx = Tx {
            micros: 33,
            settlements: Arc::new(Mutex::new(hashmap! {})),
        };
        let mut processor = UpdateCurrentPopulation::new(tx, max_abs_population_change);

        // When
        let state = block_on(processor.process(
            State::default(),
            &Instruction::UpdateCurrentPopulation(v2(1, 2)),
        ));

        // Then
        assert_eq!(state.instructions, vec![]);
    }

    #[test]
    fn should_not_change_settlement_if_last_population_update_after_game_micros() {
        // Given
        let settlement = Settlement {
            position: v2(1, 2),
            current_population: 100.0,
            target_population: 1.0,
            gap_half_life: Duration::from_micros(10),
            last_population_update_micros: 33,
            ..Settlement::default()
        };
        let settlements = Arc::new(Mutex::new(hashmap! {
            v2(1, 2) => settlement,
        }));

        let tx = Tx {
            micros: 11,
            settlements,
        };
        let mut processor = UpdateCurrentPopulation::new(tx, max_abs_population_change);

        // When
        let state = block_on(processor.process(
            State::default(),
            &Instruction::UpdateCurrentPopulation(v2(1, 2)),
        ));

        // Then
        let settlements = processor.tx.settlements.lock().unwrap();
        let settlement = settlements.get(&v2(1, 2)).unwrap();

        assert!(settlement.current_population.almost(&100.0));
        assert_eq!(settlement.last_population_update_micros, 33);
        assert_eq!(
            state.instructions,
            vec![Instruction::GetDemand(settlement.clone())]
        );
    }

    #[test]
    fn should_clamp_population_change_to_max_abs_population_change_when_increasing() {
        // Given
        let settlement = Settlement {
            position: v2(1, 2),
            current_population: 1.0,
            target_population: 100.0,
            gap_half_life: Duration::from_micros(10),
            last_population_update_micros: 11,
            ..Settlement::default()
        };
        let settlements = Arc::new(Mutex::new(hashmap! {
            v2(1, 2) => settlement,
        }));

        let tx = Tx {
            micros: 33,
            settlements,
        };
        fn max_abs_population_change(_: &SettlementClass) -> f64 {
            1.0
        };
        let mut processor = UpdateCurrentPopulation::new(tx, max_abs_population_change);

        // When
        let state = block_on(processor.process(
            State::default(),
            &Instruction::UpdateCurrentPopulation(v2(1, 2)),
        ));

        // Then
        let settlements = processor.tx.settlements.lock().unwrap();
        let settlement = settlements.get(&v2(1, 2)).unwrap();

        assert!(settlement.current_population.almost(&2.0));
        assert_eq!(settlement.last_population_update_micros, 33);
        assert_eq!(
            state.instructions,
            vec![Instruction::GetDemand(settlement.clone())]
        );
    }

    #[test]
    fn should_clamp_population_change_to_max_abs_population_change_when_decreasing() {
        // Given
        let settlement = Settlement {
            position: v2(1, 2),
            current_population: 100.0,
            target_population: 1.0,
            gap_half_life: Duration::from_micros(10),
            last_population_update_micros: 11,
            ..Settlement::default()
        };
        let settlements = Arc::new(Mutex::new(hashmap! {
            v2(1, 2) => settlement,
        }));

        let tx = Tx {
            micros: 33,
            settlements,
        };
        fn max_abs_population_change(_: &SettlementClass) -> f64 {
            1.0
        };
        let mut processor = UpdateCurrentPopulation::new(tx, max_abs_population_change);

        // When
        let state = block_on(processor.process(
            State::default(),
            &Instruction::UpdateCurrentPopulation(v2(1, 2)),
        ));

        // Then
        let settlements = processor.tx.settlements.lock().unwrap();
        let settlement = settlements.get(&v2(1, 2)).unwrap();

        assert!(settlement.current_population.almost(&99.0));
        assert_eq!(settlement.last_population_update_micros, 33);
        assert_eq!(
            state.instructions,
            vec![Instruction::GetDemand(settlement.clone())]
        );
    }
}