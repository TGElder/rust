use super::*;
use crate::game::traits::{Micros, Settlements, UpdateSettlement};
use crate::settlement::Settlement;

const HANDLE: &str = "update_current_population";

pub struct UpdateCurrentPopulation<G>
where
    G: Micros + Settlements + UpdateSettlement,
{
    game: UpdateSender<G>,
}

impl<G> Processor for UpdateCurrentPopulation<G>
where
    G: Micros + Settlements + UpdateSettlement,
{
    fn process(&mut self, mut state: State, instruction: &Instruction) -> State {
        let position = match instruction {
            Instruction::UpdateCurrentPopulation(position) => *position,
            _ => return state,
        };

        if let Some(settlement) = self.try_update_settlement(position) {
            state.instructions.push(Instruction::GetDemand(settlement))
        }

        state
    }
}

impl<G> UpdateCurrentPopulation<G>
where
    G: Micros + Settlements + UpdateSettlement,
{
    pub fn new(game: &UpdateSender<G>) -> UpdateCurrentPopulation<G> {
        UpdateCurrentPopulation {
            game: game.clone_with_handle(HANDLE),
        }
    }

    fn try_update_settlement(&mut self, position: V2<usize>) -> Option<Settlement> {
        block_on(async {
            self.game
                .update(move |game| try_update_settlement(game, position))
                .await
        })
    }
}

fn try_update_settlement<G>(game: &mut G, position: V2<usize>) -> Option<Settlement>
where
    G: Micros + Settlements + UpdateSettlement,
{
    let game_micros = *game.micros();
    let settlement = unwrap_or!(game.get_settlement(&position), return None);

    if settlement.last_population_update_micros >= game_micros {
        return Some(settlement.clone());
    }

    let new_population = get_new_population(settlement, &game_micros);

    let new_settlement = Settlement {
        current_population: new_population,
        last_population_update_micros: game_micros,
        name: settlement.name.clone(),
        nation: settlement.nation.clone(),
        ..*settlement
    };
    game.update_settlement(new_settlement.clone());
    Some(new_settlement)
}

fn get_new_population(settlement: &Settlement, game_micros: &u128) -> f64 {
    let half_life = settlement.gap_half_life.as_micros() as f64;
    if half_life == 0.0 {
        settlement.target_population
    } else {
        let last_update_micros = settlement.last_population_update_micros;
        let elapsed = (game_micros - last_update_micros) as f64;
        let exponent = elapsed / half_life;
        let decay = 0.5f64.powf(exponent);
        let gap = settlement.target_population - settlement.current_population;
        settlement.current_population + gap * decay
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use commons::almost::Almost;
    use commons::update::UpdateProcess;
    use commons::v2;
    use std::collections::HashMap;
    use std::time::Duration;

    struct MockGame {
        micros: u128,
        settlements: HashMap<V2<usize>, Settlement>,
    }

    impl Micros for MockGame {
        fn micros(&self) -> &u128 {
            &self.micros
        }
    }

    impl Settlements for MockGame {
        fn settlements(&self) -> &HashMap<V2<usize>, Settlement> {
            &self.settlements
        }
    }

    impl UpdateSettlement for MockGame {
        fn update_settlement(&mut self, settlement: Settlement) {
            self.settlements.update_settlement(settlement);
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
            ..Settlement::default()
        };
        let settlements = hashmap! {
            v2(1, 2) => settlement,
        };

        let game = MockGame {
            micros: 33,
            settlements,
        };
        let game = UpdateProcess::new(game);
        let mut processor = UpdateCurrentPopulation::new(&game.tx());

        // When
        let state = processor.process(
            State::default(),
            &Instruction::UpdateCurrentPopulation(v2(1, 2)),
        );

        // Then
        let game = game.shutdown();
        let settlement = game.get_settlement(&v2(1, 2)).unwrap();

        assert!(settlement.current_population.almost(&22.54612644157907));
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
        let settlements = hashmap! {
            v2(1, 2) => settlement,
        };

        let game = MockGame {
            micros: 33,
            settlements,
        };
        let game = UpdateProcess::new(game);
        let mut processor = UpdateCurrentPopulation::new(&game.tx());

        // When
        let state = processor.process(
            State::default(),
            &Instruction::UpdateCurrentPopulation(v2(1, 2)),
        );

        // Then
        let game = game.shutdown();
        let settlement = game.get_settlement(&v2(1, 2)).unwrap();

        assert!(settlement.current_population.almost(&78.45387355842092));
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
        let settlements = hashmap! {
            v2(1, 2) => settlement,
        };

        let game = MockGame {
            micros: 33,
            settlements,
        };
        let game = UpdateProcess::new(game);
        let mut processor = UpdateCurrentPopulation::new(&game.tx());

        // When
        let state = processor.process(
            State::default(),
            &Instruction::UpdateCurrentPopulation(v2(1, 2)),
        );

        // Then
        let game = game.shutdown();
        let settlement = game.get_settlement(&v2(1, 2)).unwrap();

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

        let game = MockGame {
            micros: 33,
            settlements: hashmap! {},
        };
        let game = UpdateProcess::new(game);
        let mut processor = UpdateCurrentPopulation::new(&game.tx());

        // When
        let state = processor.process(
            State::default(),
            &Instruction::UpdateCurrentPopulation(v2(1, 2)),
        );

        // Then
        assert_eq!(state.instructions, vec![]);

        // Finally
        game.shutdown();
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
        let settlements = hashmap! {
            v2(1, 2) => settlement,
        };

        let game = MockGame {
            micros: 11,
            settlements,
        };
        let game = UpdateProcess::new(game);
        let mut processor = UpdateCurrentPopulation::new(&game.tx());

        // When
        let state = processor.process(
            State::default(),
            &Instruction::UpdateCurrentPopulation(v2(1, 2)),
        );

        // Then
        let game = game.shutdown();
        let settlement = game.get_settlement(&v2(1, 2)).unwrap();

        assert!(settlement.current_population.almost(&100.0));
        assert_eq!(settlement.last_population_update_micros, 33);
        assert_eq!(
            state.instructions,
            vec![Instruction::GetDemand(settlement.clone())]
        );
    }
}
