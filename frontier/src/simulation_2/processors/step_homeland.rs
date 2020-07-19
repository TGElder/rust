use super::*;
use crate::game::traits::Settlements;
use crate::settlement::{Settlement, SettlementClass::Homeland};

const HANDLE: &str = "step_homeland";

pub struct StepHomeland<G>
where
    G: Settlements,
{
    game: UpdateSender<G>,
}

impl<G> Processor for StepHomeland<G>
where
    G: Settlements,
{
    fn process(&mut self, state: State, instruction: &Instruction) -> State {
        block_on(self.process(state, instruction))
    }
}

impl<G> StepHomeland<G>
where
    G: Settlements,
{
    pub fn new(game: &UpdateSender<G>) -> StepHomeland<G> {
        StepHomeland {
            game: game.clone_with_handle(HANDLE),
        }
    }

    pub async fn process(&mut self, mut state: State, instruction: &Instruction) -> State {
        match instruction {
            Instruction::Step => (),
            _ => return state,
        };
        for position in self.get_homeland_positions().await {
            state
                .instructions
                .push(Instruction::UpdateCurrentPopulation(position));
        }
        state
    }

    async fn get_homeland_positions(&mut self) -> Vec<V2<usize>> {
        self.game
            .update(|settlements| get_homeland_positions(settlements))
            .await
    }
}

fn get_homeland_positions(settlements: &dyn Settlements) -> Vec<V2<usize>> {
    settlements
        .settlements()
        .values()
        .filter(|Settlement { class, .. }| *class == Homeland)
        .map(|Settlement { position, .. }| *position)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::settlement::{SettlementClass, SettlementClass::Town};
    use commons::update::UpdateProcess;
    use commons::{same_elements, v2};
    use std::collections::HashMap;

    fn settlement(class: SettlementClass, position: V2<usize>) -> Settlement {
        Settlement {
            class,
            position,
            ..Settlement::default()
        }
    }

    #[test]
    fn should_add_update_current_population_instruction_for_each_homeland() {
        // Given
        let mut settlements = HashMap::new();
        settlements.insert(v2(1, 1), settlement(Homeland, v2(1, 1)));
        settlements.insert(v2(2, 2), settlement(Homeland, v2(2, 2)));
        let game = UpdateProcess::new(settlements);

        let mut processor = StepHomeland::new(&game.tx());

        // When
        let state = block_on(async {
            processor
                .process(State::default(), &Instruction::Step)
                .await
        });

        // Then
        assert!(same_elements(
            &state.instructions,
            &[
                Instruction::UpdateCurrentPopulation(v2(1, 1)),
                Instruction::UpdateCurrentPopulation(v2(2, 2)),
            ],
        ));

        // Finally
        game.shutdown();
    }

    #[test]
    fn should_only_add_instruction_for_homelands() {
        // Given
        let mut settlements = HashMap::new();
        settlements.insert(v2(1, 1), settlement(Town, v2(1, 1)));
        settlements.insert(v2(2, 2), settlement(Homeland, v2(2, 2)));
        let game = UpdateProcess::new(settlements);

        let mut processor = StepHomeland::new(&game.tx());

        // When
        let state = block_on(async {
            processor
                .process(State::default(), &Instruction::Step)
                .await
        });

        // Then
        assert!(same_elements(
            &state.instructions,
            &[Instruction::UpdateCurrentPopulation(v2(2, 2)),],
        ));

        // Finally
        game.shutdown();
    }
}
