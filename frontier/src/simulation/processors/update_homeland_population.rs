use super::*;
use crate::game::traits::{Settlements, UpdateSettlement, VisiblePositions};
use crate::settlement::{Settlement, SettlementClass::Homeland};

const NAME: &str = "update_homeland_population";

pub struct UpdateHomelandPopulation<G>
where
    G: Settlements + UpdateSettlement + VisiblePositions + Send,
{
    game: FnSender<G>,
}

#[async_trait]
impl<G> Processor for UpdateHomelandPopulation<G>
where
    G: Settlements + UpdateSettlement + VisiblePositions + Send,
{
    async fn process(&mut self, state: State, instruction: &Instruction) -> State {
        match instruction {
            Instruction::VisibleLandPositions => (),
            _ => return state,
        };
        let visibile_land_positions = self.visibile_land_positions().await;
        self.update_homelands(visibile_land_positions as f64).await;
        state
    }
}

impl<G> UpdateHomelandPopulation<G>
where
    G: Settlements + UpdateSettlement + VisiblePositions + Send,
{
    pub fn new(game: &FnSender<G>) -> UpdateHomelandPopulation<G> {
        UpdateHomelandPopulation {
            game: game.clone_with_name(NAME),
        }
    }

    async fn visibile_land_positions(&mut self) -> usize {
        self.game.send(|game| visible_land_positions(game)).await
    }

    async fn update_homelands(&mut self, total_population: f64) {
        self.game
            .send(move |game| update_homelands(game, total_population))
            .await
    }
}

fn visible_land_positions<G>(game: &mut G) -> usize
where
    G: VisiblePositions + Send,
{
    game.visible_land_positions()
}

fn update_homelands<G>(game: &mut G, total_population: f64)
where
    G: Settlements + UpdateSettlement + Send,
{
    let homelands = get_homelands(game);
    let target_population = total_population / homelands.len() as f64;
    for homeland in homelands {
        update_homeland(game, homeland, target_population);
    }
}

fn get_homelands<G>(game: &mut G) -> Vec<Settlement>
where
    G: Settlements,
{
    game.settlements()
        .values()
        .filter(|settlement| settlement.class == Homeland)
        .cloned()
        .collect()
}

fn update_homeland<G>(game: &mut G, settlement: Settlement, target_population: f64)
where
    G: UpdateSettlement,
{
    let settlement = Settlement {
        target_population,
        ..settlement
    };
    game.update_settlement(settlement);
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    use commons::fn_sender::FnThread;
    use commons::futures::executor::block_on;
    use commons::v2;

    struct MockGame {
        settlements: HashMap<V2<usize>, Settlement>,
        visible_land_positions: usize,
    }

    impl Settlements for MockGame {
        fn settlements(&self) -> &HashMap<V2<usize>, Settlement> {
            &self.settlements
        }
    }

    impl UpdateSettlement for MockGame {
        fn update_settlement(&mut self, settlement: Settlement) {
            self.settlements.insert(settlement.position, settlement);
        }
    }

    impl VisiblePositions for MockGame {
        fn visible_land_positions(&self) -> usize {
            self.visible_land_positions
        }
    }

    #[test]
    fn each_homeland_population_should_be_equal_share_of_visible_land() {
        // Given
        let settlements = hashmap! {
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
        };
        let game = FnThread::new(MockGame {
            settlements,
            visible_land_positions: 202,
        });
        let mut processor = UpdateHomelandPopulation::new(&game.tx());

        // When
        block_on(processor.process(State::default(), &Instruction::VisibleLandPositions));

        // Then
        let actual = game.join().settlements;
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
        assert_eq!(actual, expected);
    }
}
