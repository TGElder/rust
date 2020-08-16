use super::*;
use crate::game::traits::UpdateSettlement;
use crate::settlement::Settlement;

const HANDLE: &str = "update_town_population";

pub struct UpdateTownPopulation<G>
where
    G: UpdateSettlement,
{
    game: UpdateSender<G>,
}

impl<G> Processor for UpdateTownPopulation<G>
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
        if let Some(target_population) =
            get_target_population(traffic, state.params.traffic_to_population)
        {
            self.update_settlement(Settlement {
                target_population,
                ..settlement.clone()
            })
        }
        state
    }
}

impl<G> UpdateTownPopulation<G>
where
    G: UpdateSettlement,
{
    pub fn new(game: &UpdateSender<G>) -> UpdateTownPopulation<G> {
        UpdateTownPopulation {
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
) -> Option<f64> {
    if traffic_summaries.is_empty() {
        return None;
    }
    let total_traffic_share: f64 = traffic_summaries
        .iter()
        .map(|traffic_summary| traffic_summary.traffic_share)
        .sum();
    Some(total_traffic_share * traffic_to_population)
}

#[cfg(test)]
mod tests {
    use super::*;

    use commons::update::UpdateProcess;
    use commons::v2;

    use std::default::Default;

    #[test]
    fn should_update_town_target_population_based_on_total_traffic_share() {
        // Given
        let settlement = Settlement::default();
        let game = UpdateProcess::new(hashmap! {});
        let mut processor = UpdateTownPopulation::new(&game.tx());

        // When
        let instruction = Instruction::UpdateTown {
            settlement: settlement.clone(),
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
        assert_eq!(
            updated_settlements[&v2(0, 0)],
            Settlement {
                target_population: 28.0,
                ..settlement
            }
        );
    }

    #[test]
    fn should_ignore_town_with_no_traffic() {
        // Given
        let settlement = Settlement::default();
        let game = UpdateProcess::new(hashmap! {});
        let mut processor = UpdateTownPopulation::new(&game.tx());

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
