use crate::settlement::{Settlement, SettlementClass};
use crate::simulation::settlement::SettlementSimulation;
use crate::simulation::MaxAbsPopulationChange;
use crate::traits::has::HasParameters;
use crate::traits::{Micros, UpdateSettlement};

impl<T> SettlementSimulation<T>
where
    T: HasParameters + Micros + UpdateSettlement,
{
    pub async fn update_current_population(&self, settlement: Settlement) -> Settlement {
        self.try_update_settlement(settlement).await
    }

    async fn try_update_settlement(&self, settlement: Settlement) -> Settlement {
        let game_micros = self.tx.micros().await;

        if settlement.last_population_update_micros >= game_micros {
            return settlement;
        }

        let change = clamp_population_change(
            get_population_change(&settlement, &game_micros),
            self.max_abs_population_change(&settlement.class).await,
        );
        let current_population = settlement.current_population + change;

        let new_settlement = Settlement {
            current_population,
            last_population_update_micros: game_micros,
            ..settlement
        };
        self.tx.update_settlement(new_settlement.clone()).await;
        new_settlement
    }

    async fn max_abs_population_change(&self, settlement_class: &SettlementClass) -> f64 {
        let MaxAbsPopulationChange { homeland, town } =
            self.tx.parameters().simulation.max_abs_population_change;
        match settlement_class {
            SettlementClass::Homeland => homeland,
            SettlementClass::Town => town,
        }
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

#[cfg(test)]
mod tests {
    use crate::parameters::Parameters;
    use crate::settlement::SettlementClass::Town;

    use super::*;

    use commons::almost::Almost;
    use commons::async_trait::async_trait;
    use commons::{v2, Arm, V2};
    use futures::executor::block_on;
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::time::Duration;

    struct Tx {
        micros: u128,
        parameters: Parameters,
        settlements: Arm<HashMap<V2<usize>, Settlement>>,
    }

    impl HasParameters for Tx {
        fn parameters(&self) -> &Parameters {
            &self.parameters
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

    fn tx() -> Tx {
        let mut parameters = Parameters::default();
        parameters.simulation.max_abs_population_change = MaxAbsPopulationChange {
            town: 100.0,
            homeland: 0.0,
        };
        Tx {
            micros: 33,
            parameters,
            settlements: Arc::default(),
        }
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
            class: Town,
            ..Settlement::default()
        };
        let sim = SettlementSimulation::new(tx());

        // When
        let result = block_on(sim.update_current_population(settlement));

        // Then
        let settlements = sim.tx.settlements.lock().unwrap();
        let settlement = settlements.get(&v2(1, 2)).unwrap();

        assert!(settlement.current_population.almost(&78.45387355842092));
        assert_eq!(settlement.last_population_update_micros, 33);
        assert_eq!(result, settlement.clone());
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
            class: Town,
            ..Settlement::default()
        };
        let sim = SettlementSimulation::new(tx());

        // When
        let result = block_on(sim.update_current_population(settlement));

        // Then
        let settlements = sim.tx.settlements.lock().unwrap();
        let settlement = settlements.get(&v2(1, 2)).unwrap().clone();

        assert!(settlement.current_population.almost(&22.54612644157907));
        assert_eq!(settlement.last_population_update_micros, 33);
        assert_eq!(result, settlement);
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
            class: Town,
            ..Settlement::default()
        };
        let sim = SettlementSimulation::new(tx());

        // When
        let result = block_on(sim.update_current_population(settlement));

        // Then
        let settlements = sim.tx.settlements.lock().unwrap();
        let settlement = settlements.get(&v2(1, 2)).unwrap();

        assert!(settlement
            .current_population
            .almost(&settlement.target_population));
        assert_eq!(settlement.last_population_update_micros, 33);
        assert_eq!(result, settlement.clone());
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
            class: Town,
            ..Settlement::default()
        };
        let tx = Tx { micros: 11, ..tx() };
        let sim = SettlementSimulation::new(tx);

        // When
        let result = block_on(sim.update_current_population(settlement.clone()));

        // Then
        let settlements = sim.tx.settlements.lock().unwrap();

        assert_eq!(settlements.get(&v2(1, 2)), None);
        assert_eq!(result, settlement);
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
            class: Town,
            ..Settlement::default()
        };
        let mut tx = tx();
        tx.parameters.simulation.max_abs_population_change.town = 1.0;
        let sim = SettlementSimulation::new(tx);

        // When
        let result = block_on(sim.update_current_population(settlement));

        // Then
        let settlements = sim.tx.settlements.lock().unwrap();
        let settlement = settlements.get(&v2(1, 2)).unwrap();
        assert!(settlement.current_population.almost(&2.0));
        assert_eq!(settlement.last_population_update_micros, 33);
        assert_eq!(result, settlement.clone());
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
            class: Town,
            ..Settlement::default()
        };
        let mut tx = tx();
        tx.parameters.simulation.max_abs_population_change.town = 1.0;
        let sim = SettlementSimulation::new(tx);

        // When
        let result = block_on(sim.update_current_population(settlement));

        // Then
        let settlements = sim.tx.settlements.lock().unwrap();
        let settlement = settlements.get(&v2(1, 2)).unwrap();

        assert!(settlement.current_population.almost(&99.0));
        assert_eq!(settlement.last_population_update_micros, 33);
        assert_eq!(result, settlement.clone());
    }
}
