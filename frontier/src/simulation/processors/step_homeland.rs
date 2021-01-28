use std::collections::HashMap;

use super::*;
use crate::settlement::{Settlement, SettlementClass::Homeland};
use crate::traits::SendSettlements;

pub struct StepHomeland<T> {
    tx: T,
}

#[async_trait]
impl<T> Processor for StepHomeland<T>
where
    T: SendSettlements + Send + Sync,
{
    async fn process(&mut self, state: State, instruction: &Instruction) -> State {
        self.process(state, instruction).await
    }
}

impl<T> StepHomeland<T>
where
    T: SendSettlements,
{
    pub fn new(tx: T) -> StepHomeland<T> {
        StepHomeland { tx }
    }

    pub async fn process(&mut self, mut state: State, instruction: &Instruction) -> State {
        match instruction {
            Instruction::Step => (),
            _ => return state,
        };
        for position in self.get_homeland_positions().await {
            state.instructions.push(Instruction::Build);
            state
                .instructions
                .push(Instruction::UpdateCurrentPopulation(position));
        }
        state
            .instructions
            .push(Instruction::UpdateHomelandPopulation);
        state
    }

    async fn get_homeland_positions(&mut self) -> Vec<V2<usize>> {
        self.tx
            .send_settlements(|settlements| get_homeland_positions(settlements))
            .await
    }
}

fn get_homeland_positions(settlements: &HashMap<V2<usize>, Settlement>) -> Vec<V2<usize>> {
    settlements
        .values()
        .filter(|Settlement { class, .. }| *class == Homeland)
        .map(|Settlement { position, .. }| *position)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::settlement::{SettlementClass, SettlementClass::Town};
    use commons::{same_elements, v2};
    use futures::executor::block_on;
    use std::collections::HashMap;
    use std::sync::Mutex;

    fn settlement(class: SettlementClass, position: V2<usize>) -> Settlement {
        Settlement {
            class,
            position,
            ..Settlement::default()
        }
    }

    #[test]
    fn should_add_instructions_for_each_homeland() {
        // Given
        let mut settlements = HashMap::new();
        settlements.insert(v2(1, 1), settlement(Homeland, v2(1, 1)));
        settlements.insert(v2(2, 2), settlement(Homeland, v2(2, 2)));

        let mut processor = StepHomeland::new(Mutex::new(settlements));

        // When
        let state = block_on(processor.process(State::default(), &Instruction::Step));

        // Then
        assert!(same_elements(
            &state.instructions,
            &[
                Instruction::Build,
                Instruction::UpdateCurrentPopulation(v2(1, 1)),
                Instruction::Build,
                Instruction::UpdateCurrentPopulation(v2(2, 2)),
                Instruction::UpdateHomelandPopulation
            ],
        ));
    }

    #[test]
    fn should_only_add_instruction_for_homelands() {
        // Given
        let mut settlements = HashMap::new();
        settlements.insert(v2(1, 1), settlement(Town, v2(1, 1)));
        settlements.insert(v2(2, 2), settlement(Homeland, v2(2, 2)));

        let mut processor = StepHomeland::new(Mutex::new(settlements));

        // When
        let state = block_on(processor.process(State::default(), &Instruction::Step));

        // Then
        assert_eq!(
            state.instructions,
            vec![
                Instruction::Build,
                Instruction::UpdateCurrentPopulation(v2(2, 2)),
                Instruction::UpdateHomelandPopulation
            ],
        );
    }
}
