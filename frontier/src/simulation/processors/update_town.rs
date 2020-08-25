use super::*;
use crate::game::traits::UpdateSettlement;
use crate::settlement::Settlement;
use commons::unsafe_ordering;

const HANDLE: &str = "update_town";

pub struct UpdateTown<G>
where
    G: UpdateSettlement,
{
    game: UpdateSender<G>,
}

impl<G> Processor for UpdateTown<G>
where
    G: UpdateSettlement,
{
    fn process(&mut self, mut state: State, instruction: &Instruction) -> State {
        let (settlement, traffic) = match instruction {
            Instruction::UpdateTown {
                settlement,
                traffic,
            } => (settlement, traffic),
            _ => return state,
        };

        self.update_settlement(Settlement {
            target_population: get_target_population(traffic, state.params.traffic_to_population),
            nation: get_nation(
                &settlement.nation,
                traffic,
                state.params.nation_flip_traffic_pc,
            ),
            ..settlement.clone()
        });

        state
            .instructions
            .push(Instruction::UpdateCurrentPopulation(settlement.position));

        state
    }
}

impl<G> UpdateTown<G>
where
    G: UpdateSettlement,
{
    pub fn new(game: &UpdateSender<G>) -> UpdateTown<G> {
        UpdateTown {
            game: game.clone_with_handle(HANDLE),
        }
    }

    fn update_settlement(&mut self, settlement: Settlement) {
        block_on(async {
            self.game
                .update(move |game| game.update_settlement(settlement))
                .await
        });
    }
}

fn get_target_population(
    traffic_summaries: &[TownTrafficSummary],
    traffic_to_population: f64,
) -> f64 {
    let total_traffic_share: f64 = traffic_summaries
        .iter()
        .map(|traffic_summary| traffic_summary.traffic_share)
        .sum();
    total_traffic_share * traffic_to_population
}

fn get_nation(
    original_nation: &str,
    traffic_summaries: &[TownTrafficSummary],
    nation_flip_traffic_pc: f64,
) -> String {
    let total_traffic_share: f64 = traffic_summaries
        .iter()
        .map(|traffic_summary| traffic_summary.traffic_share)
        .sum();
    if total_traffic_share == 0.0 {
        return original_nation.to_string();
    }
    let traffic_summary = traffic_summaries
        .iter()
        .max_by(|a, b| unsafe_ordering(&a.traffic_share, &b.traffic_share))
        .unwrap();
    if traffic_summary.traffic_share / total_traffic_share >= nation_flip_traffic_pc {
        traffic_summary.nation.clone()
    } else {
        original_nation.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use commons::almost::Almost;
    use commons::update::UpdateProcess;
    use commons::v2;

    use std::default::Default;

    #[test]
    fn should_update_target_population_based_on_total_traffic_share() {
        // Given
        let settlement = Settlement::default();
        let game = UpdateProcess::new(hashmap! {});
        let mut processor = UpdateTown::new(&game.tx());

        // When
        let instruction = Instruction::UpdateTown {
            settlement,
            traffic: vec![
                TownTrafficSummary {
                    nation: "A".to_string(),
                    traffic_share: 17.0,
                },
                TownTrafficSummary {
                    nation: "B".to_string(),
                    traffic_share: 39.0,
                },
            ],
        };
        let state = State {
            params: SimulationParams {
                traffic_to_population: 0.5,
                ..SimulationParams::default()
            },
            ..State::default()
        };
        processor.process(state, &instruction);

        // Then
        let updated_settlements = game.shutdown();
        assert!(updated_settlements[&v2(0, 0)]
            .target_population
            .almost(&28.0));
    }

    #[test]
    fn should_update_target_population_to_zero_for_town_with_no_traffic() {
        // Given
        let settlement = Settlement {
            target_population: 0.5,
            ..Settlement::default()
        };
        let game = UpdateProcess::new(hashmap! {});
        let mut processor = UpdateTown::new(&game.tx());

        // When
        let instruction = Instruction::UpdateTown {
            settlement,
            traffic: vec![],
        };
        processor.process(State::default(), &instruction);

        // Then
        let updated_settlements = game.shutdown();
        assert!(updated_settlements[&v2(0, 0)]
            .target_population
            .almost(&0.0));
    }

    #[test]
    fn should_update_town_nation_if_any_nation_exceeds_nation_flip_traffic_pc() {
        // Given
        let settlement = Settlement {
            nation: "A".to_string(),
            ..Settlement::default()
        };
        let game = UpdateProcess::new(hashmap! {});
        let mut processor = UpdateTown::new(&game.tx());

        // When
        let instruction = Instruction::UpdateTown {
            settlement,
            traffic: vec![
                TownTrafficSummary {
                    nation: "B".to_string(),
                    traffic_share: 32.0,
                },
                TownTrafficSummary {
                    nation: "C".to_string(),
                    traffic_share: 68.0,
                },
            ],
        };
        let state = State {
            params: SimulationParams {
                nation_flip_traffic_pc: 0.67,
                ..SimulationParams::default()
            },
            ..State::default()
        };
        processor.process(state, &instruction);

        // Then
        let updated_settlements = game.shutdown();
        assert_eq!(updated_settlements[&v2(0, 0)].nation, "C".to_string(),);
    }

    #[test]
    fn should_keep_original_nation_if_no_nation_exceeds_nation_flip_traffic_pc() {
        // Given
        let settlement = Settlement {
            nation: "A".to_string(),
            ..Settlement::default()
        };
        let game = UpdateProcess::new(hashmap! {});
        let mut processor = UpdateTown::new(&game.tx());

        // When
        let instruction = Instruction::UpdateTown {
            settlement,
            traffic: vec![
                TownTrafficSummary {
                    nation: "B".to_string(),
                    traffic_share: 40.0,
                },
                TownTrafficSummary {
                    nation: "C".to_string(),
                    traffic_share: 60.0,
                },
            ],
        };
        let state = State {
            params: SimulationParams {
                nation_flip_traffic_pc: 0.67,
                ..SimulationParams::default()
            },
            ..State::default()
        };
        processor.process(state, &instruction);

        // Then
        let updated_settlements = game.shutdown();
        assert_eq!(updated_settlements[&v2(0, 0)].nation, "A".to_string());
    }

    #[test]
    fn should_add_update_current_population_instruction() {
        // Given
        let settlement = Settlement::default();
        let game = UpdateProcess::new(hashmap! {});
        let mut processor = UpdateTown::new(&game.tx());

        // When
        let instruction = Instruction::UpdateTown {
            settlement: settlement.clone(),
            traffic: vec![TownTrafficSummary {
                nation: "A".to_string(),
                traffic_share: 1.0,
            }],
        };
        let state = processor.process(State::default(), &instruction);

        // Then
        assert_eq!(
            state.instructions,
            vec![Instruction::UpdateCurrentPopulation(settlement.position)]
        );

        // Finally
        game.shutdown();
    }
}