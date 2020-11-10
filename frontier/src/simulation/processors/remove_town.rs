use super::*;
use crate::game::traits::{Controlled, RemoveSettlement};
use std::collections::HashSet;

const HANDLE: &str = "remove_town";

pub struct RemoveTown<G>
where
    G: RemoveSettlement + Controlled,
{
    game: FnSender<G>,
}

#[async_trait]
impl<G> Processor for RemoveTown<G>
where
    G: RemoveSettlement + Controlled,
{
    async fn process(&mut self, mut state: State, instruction: &Instruction) -> State {
        let (settlement, traffic) = match instruction {
            Instruction::UpdateTown {
                settlement,
                traffic,
            } => (settlement, traffic),
            _ => return state,
        };
        if settlement.current_population >= state.params.town_removal_population
            || !traffic.is_empty()
        {
            return state;
        }
        let controlled = self
            .remove_settlement_and_return_controlled(settlement.position)
            .await;
        state
            .instructions
            .push(Instruction::RefreshPositions(controlled));
        state
    }
}

impl<G> RemoveTown<G>
where
    G: RemoveSettlement + Controlled,
{
    pub fn new(game: &FnSender<G>) -> RemoveTown<G> {
        RemoveTown {
            game: game.clone_with_name(HANDLE),
        }
    }

    async fn remove_settlement_and_return_controlled(
        &mut self,
        position: V2<usize>,
    ) -> HashSet<V2<usize>> {
        self.game
            .send(move |game| remove_settlement_and_return_controlled(game, position))
            .await
    }
}

fn remove_settlement_and_return_controlled<G>(
    game: &mut G,
    position: V2<usize>,
) -> HashSet<V2<usize>>
where
    G: RemoveSettlement + Controlled,
{
    let out = game.controlled(&position);
    game.remove_settlement(&position);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::settlement::Settlement;
    use commons::fn_sender::FnThread;
    use commons::futures::executor::block_on;
    use commons::v2;
    use std::collections::HashSet;
    use std::default::Default;

    struct MockGame {
        controlled: HashSet<V2<usize>>,
        removed: Vec<V2<usize>>,
    }

    impl Default for MockGame {
        fn default() -> MockGame {
            MockGame {
                controlled: hashset! {},
                removed: vec![],
            }
        }
    }

    impl Controlled for MockGame {
        fn controlled(&self, _: &V2<usize>) -> HashSet<V2<usize>> {
            self.controlled.clone()
        }
    }

    impl RemoveSettlement for MockGame {
        fn remove_settlement(&mut self, position: &V2<usize>) {
            self.removed.push(*position);
        }
    }

    #[test]
    fn should_remove_town_with_no_traffic_and_current_population_below_threshold() {
        // Given
        let settlement = Settlement {
            current_population: 0.2,
            ..Settlement::default()
        };
        let game = FnThread::new(MockGame::default());
        let mut processor = RemoveTown::new(&game.tx());
        let state = State {
            params: SimulationParams {
                town_removal_population: 0.5,
                ..SimulationParams::default()
            },
            ..State::default()
        };

        // When
        let instruction = Instruction::UpdateTown {
            settlement: settlement.clone(),
            traffic: vec![],
        };
        block_on(processor.process(state, &instruction));

        // Then
        let game = game.join();
        assert_eq!(game.removed, vec![settlement.position]);
    }

    #[test]
    fn should_not_remove_town_with_current_population_below_threshold_but_traffic() {
        // Given
        let settlement = Settlement {
            current_population: 0.2,
            ..Settlement::default()
        };
        let game = FnThread::new(MockGame::default());
        let mut processor = RemoveTown::new(&game.tx());
        let state = State {
            params: SimulationParams {
                town_removal_population: 0.5,
                ..SimulationParams::default()
            },
            ..State::default()
        };

        // When
        let instruction = Instruction::UpdateTown {
            settlement,
            traffic: vec![TownTrafficSummary {
                nation: "A".to_string(),
                traffic_share: 1.0,
            }],
        };
        block_on(processor.process(state, &instruction));

        // Then
        let game = game.join();
        assert_eq!(game.removed, vec![]);
    }

    #[test]
    fn should_not_remove_town_with_no_traffic_but_current_population_above_threshold() {
        // Given
        let settlement = Settlement {
            current_population: 0.7,
            ..Settlement::default()
        };
        let game = FnThread::new(MockGame::default());
        let mut processor = RemoveTown::new(&game.tx());
        let state = State {
            params: SimulationParams {
                town_removal_population: 0.5,
                ..SimulationParams::default()
            },
            ..State::default()
        };

        // When
        let instruction = Instruction::UpdateTown {
            settlement,
            traffic: vec![],
        };
        block_on(processor.process(state, &instruction));

        // Then
        let game = game.join();
        assert_eq!(game.removed, vec![]);
    }

    #[test]
    fn should_refresh_all_positions_controlled_by_town() {
        // Given
        let settlement = Settlement {
            current_population: 0.2,
            ..Settlement::default()
        };
        let game = MockGame {
            controlled: hashset! { v2(1, 2), v2(3, 4) },
            ..MockGame::default()
        };
        let game = FnThread::new(game);
        let mut processor = RemoveTown::new(&game.tx());
        let state = State {
            params: SimulationParams {
                town_removal_population: 0.5,
                ..SimulationParams::default()
            },
            ..State::default()
        };

        // When
        let instruction = Instruction::UpdateTown {
            settlement,
            traffic: vec![],
        };
        let state = block_on(processor.process(state, &instruction));

        assert_eq!(
            state.instructions,
            vec![Instruction::RefreshPositions(
                hashset! { v2(1, 2), v2(3, 4) },
            )]
        );

        // Finally
        game.join();
    }
}
