use super::*;
use crate::game::traits::{Settlements, UpdateSettlement};
use crate::settlement::{Settlement, SettlementClass::Homeland};

const HANDLE: &str = "homeland_target_population";

pub struct HomelandTargetPopulation<G>
where
    G: Settlements + UpdateSettlement,
{
    game: UpdateSender<G>,
}

impl<G> Processor for HomelandTargetPopulation<G>
where
    G: Settlements + UpdateSettlement,
{
    fn process(&mut self, state: State, instruction: &Instruction) -> State {
        let visible_land = match instruction {
            Instruction::VisibleLandPositions(visible_land) => visible_land,
            _ => return state,
        };
        self.update_homelands(*visible_land as f64);
        state
    }
}

impl<G> HomelandTargetPopulation<G>
where
    G: Settlements + UpdateSettlement,
{
    pub fn new(game: &UpdateSender<G>) -> HomelandTargetPopulation<G> {
        HomelandTargetPopulation {
            game: game.clone_with_handle(HANDLE),
        }
    }

    fn update_homelands(&mut self, total_population: f64) {
        block_on(async {
            self.game
                .update(move |game| update_homelands(game, total_population))
                .await
        })
    }
}

fn update_homelands<G>(game: &mut G, total_population: f64)
where
    G: Settlements + UpdateSettlement,
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
    use super::*;

    use commons::update::UpdateProcess;
    use commons::v2;

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
        let game = UpdateProcess::new(settlements);
        let mut processor = HomelandTargetPopulation::new(&game.tx());

        // When
        processor.process(State::default(), &Instruction::VisibleLandPositions(202));

        // Then
        let actual = game.shutdown();
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
