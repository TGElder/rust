use super::*;
use crate::game::traits::UpdateSettlement;
use crate::settlement::Settlement;
use commons::unsafe_ordering;

const HANDLE: &str = "update_town_nation";

pub struct UpdateTownNation<G>
where
    G: UpdateSettlement,
{
    game: UpdateSender<G>,
}

impl<G> Processor for UpdateTownNation<G>
where
    G: UpdateSettlement,
{
    fn process(&mut self, state: State, instruction: &Instruction) -> State {
        let (settlement, traffic) = match instruction {
            Instruction::UpdateTown {
                settlement,
                traffic,
            } => (settlement, traffic),
            _ => return state,
        };
        if let Some(nation) = get_nation(traffic, state.params.nation_flip_traffic_pc) {
            if nation != settlement.nation {
                self.update_settlement(Settlement {
                    nation,
                    ..settlement.clone()
                })
            }
        }
        state
    }
}

impl<G> UpdateTownNation<G>
where
    G: UpdateSettlement,
{
    pub fn new(game: &UpdateSender<G>) -> UpdateTownNation<G> {
        UpdateTownNation {
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

fn get_nation(
    traffic_summaries: &[TownTrafficSummary],
    nation_flip_traffic_pc: f64,
) -> Option<String> {
    if traffic_summaries.is_empty() {
        return None;
    }
    let total_traffic_share: f64 = traffic_summaries
        .iter()
        .map(|traffic_summary| traffic_summary.traffic_share)
        .sum();
    let traffic_summary = traffic_summaries
        .iter()
        .max_by(|a, b| unsafe_ordering(&a.traffic_share, &b.traffic_share))?;
    if traffic_summary.traffic_share / total_traffic_share >= nation_flip_traffic_pc {
        Some(traffic_summary.nation.clone())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use commons::update::UpdateProcess;
    use commons::v2;

    use std::default::Default;

    #[test]
    fn should_update_town_nation_if_any_nation_exceeds_nation_flip_traffic_pc() {
        // Given
        let settlement = Settlement::default();
        let game = UpdateProcess::new(hashmap! {});
        let mut processor = UpdateTownNation::new(&game.tx());

        // When
        let instruction = Instruction::UpdateTown {
            settlement: settlement.clone(),
            traffic: vec![
                TownTrafficSummary {
                    nation: "A".to_string(),
                    traffic_share: 32.0,
                },
                TownTrafficSummary {
                    nation: "B".to_string(),
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
        assert_eq!(
            updated_settlements[&v2(0, 0)],
            Settlement {
                nation: "B".to_string(),
                ..settlement
            }
        );
    }

    #[test]
    fn should_do_nothing_if_no_nation_exceeds_nation_flip_traffic_pc() {
        // Given
        let settlement = Settlement::default();
        let game = UpdateProcess::new(hashmap! {});
        let mut processor = UpdateTownNation::new(&game.tx());

        // When
        let instruction = Instruction::UpdateTown {
            settlement,
            traffic: vec![
                TownTrafficSummary {
                    nation: "A".to_string(),
                    traffic_share: 40.0,
                },
                TownTrafficSummary {
                    nation: "B".to_string(),
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
        assert!(updated_settlements.is_empty());
    }

    #[test]
    fn should_do_nothing_if_nation_is_unchaged() {
        // Given
        let settlement = Settlement {
            nation: "A".to_string(),
            ..Settlement::default()
        };
        let game = UpdateProcess::new(hashmap! {});
        let mut processor = UpdateTownNation::new(&game.tx());

        // When
        let instruction = Instruction::UpdateTown {
            settlement,
            traffic: vec![
                TownTrafficSummary {
                    nation: "A".to_string(),
                    traffic_share: 68.0,
                },
                TownTrafficSummary {
                    nation: "B".to_string(),
                    traffic_share: 32.0,
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
        assert!(updated_settlements.is_empty());
    }

    #[test]
    fn should_ignore_town_with_no_traffic() {
        // Given
        let settlement = Settlement::default();
        let game = UpdateProcess::new(hashmap! {});
        let mut processor = UpdateTownNation::new(&game.tx());

        // When
        let instruction = Instruction::UpdateTown {
            settlement,
            traffic: vec![],
        };
        processor.process(State::default(), &instruction);

        // Then
        let updated_settlements = game.shutdown();
        assert!(updated_settlements.is_empty());
    }
}
