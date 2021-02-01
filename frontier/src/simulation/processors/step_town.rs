use std::collections::HashMap;

use super::*;
use crate::settlement::{Settlement, SettlementClass::Town};
use crate::traits::SendSettlements;

pub struct StepTown<T> {
    tx: T,
}

#[async_trait]
impl<T> Processor for StepTown<T>
where
    T: SendSettlements + Send + Sync,
{
    async fn process(&mut self, state: State, instruction: &Instruction) -> State {
        self.process(state, instruction).await
    }
}

impl<T> StepTown<T>
where
    T: SendSettlements,
{
    pub fn new(tx: T) -> StepTown<T> {
        StepTown { tx }
    }

    pub async fn process(&mut self, mut state: State, instruction: &Instruction) -> State {
        match instruction {
            Instruction::Step => (),
            _ => return state,
        };
        for position in self.get_town_positions().await {
            state.instructions.push(Instruction::GetTerritory(position));
        }
        state
    }

    async fn get_town_positions(&mut self) -> Vec<V2<usize>> {
        self.tx
            .send_settlements(|settlements| get_town_positions(settlements))
            .await
    }
}

fn get_town_positions(settlements: &HashMap<V2<usize>, Settlement>) -> Vec<V2<usize>> {
    settlements
        .values()
        .filter(|Settlement { class, .. }| *class == Town)
        .map(|Settlement { position, .. }| *position)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::settlement::{SettlementClass, SettlementClass::Homeland};
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
    fn should_add_instructions_for_each_town() {
        // Given
        let mut settlements = HashMap::new();
        settlements.insert(v2(1, 1), settlement(Town, v2(1, 1)));
        settlements.insert(v2(2, 2), settlement(Town, v2(2, 2)));

        let mut processor = StepTown::new(Mutex::new(settlements));

        // When
        let state = block_on(processor.process(State::default(), &Instruction::Step));

        // Then
        assert!(same_elements(
            &state.instructions,
            &[
                Instruction::GetTerritory(v2(1, 1)),
                Instruction::GetTerritory(v2(2, 2)),
            ],
        ));
    }

    #[test]
    fn should_only_add_instruction_for_towns() {
        // Given
        let mut settlements = HashMap::new();
        settlements.insert(v2(1, 1), settlement(Homeland, v2(1, 1)));
        settlements.insert(v2(2, 2), settlement(Town, v2(2, 2)));

        let mut processor = StepTown::new(Mutex::new(settlements));

        // When
        let state = block_on(processor.process(State::default(), &Instruction::Step));

        // Then
        assert_eq!(
            state.instructions,
            vec![Instruction::GetTerritory(v2(2, 2)),]
        );
    }
}
