use super::*;
use crate::game::traits::Settlements;
use crate::settlement::{Settlement, SettlementClass::Town};

const HANDLE: &str = "step_town";

pub struct StepTown<G>
where
    G: Settlements,
{
    game: UpdateSender<G>,
}

#[async_trait]
impl<G> Processor for StepTown<G>
where
    G: Settlements,
{
    async fn process(&mut self, state: State, instruction: &Instruction) -> State {
        self.process(state, instruction).await
    }
}

impl<G> StepTown<G>
where
    G: Settlements,
{
    pub fn new(game: &UpdateSender<G>) -> StepTown<G> {
        StepTown {
            game: game.clone_with_handle(HANDLE),
        }
    }

    pub async fn process(&mut self, mut state: State, instruction: &Instruction) -> State {
        match instruction {
            Instruction::Step => (),
            _ => return state,
        };
        for position in self.get_town_positions().await {
            state.instructions.push(Instruction::Build);
            state.instructions.push(Instruction::GetTerritory(position));
        }
        state
    }

    async fn get_town_positions(&mut self) -> Vec<V2<usize>> {
        self.game
            .update(|settlements| get_town_positions(settlements))
            .await
    }
}

fn get_town_positions(settlements: &dyn Settlements) -> Vec<V2<usize>> {
    settlements
        .settlements()
        .values()
        .filter(|Settlement { class, .. }| *class == Town)
        .map(|Settlement { position, .. }| *position)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::settlement::{SettlementClass, SettlementClass::Homeland};
    use commons::futures::executor::block_on;
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
    fn should_add_instructions_for_each_town() {
        // Given
        let mut settlements = HashMap::new();
        settlements.insert(v2(1, 1), settlement(Town, v2(1, 1)));
        settlements.insert(v2(2, 2), settlement(Town, v2(2, 2)));
        let game = UpdateProcess::new(settlements);

        let mut processor = StepTown::new(&game.tx());

        // When
        let state = block_on(processor.process(State::default(), &Instruction::Step));

        // Then
        assert!(same_elements(
            &state.instructions,
            &[
                Instruction::Build,
                Instruction::GetTerritory(v2(1, 1)),
                Instruction::Build,
                Instruction::GetTerritory(v2(2, 2)),
            ],
        ));

        // Finally
        game.shutdown();
    }

    #[test]
    fn should_only_add_instruction_for_towns() {
        // Given
        let mut settlements = HashMap::new();
        settlements.insert(v2(1, 1), settlement(Homeland, v2(1, 1)));
        settlements.insert(v2(2, 2), settlement(Town, v2(2, 2)));
        let game = UpdateProcess::new(settlements);

        let mut processor = StepTown::new(&game.tx());

        // When
        let state = block_on(processor.process(State::default(), &Instruction::Step));

        // Then
        assert_eq!(
            state.instructions,
            vec![Instruction::Build, Instruction::GetTerritory(v2(2, 2)),]
        );

        // Finally
        game.shutdown();
    }
}
