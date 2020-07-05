use super::*;
use crate::game::traits::Settlements;
use crate::settlement::Settlement;

const HANDLE: &str = "settlement_ref_to_settlement";

pub struct SettlementRefToSettlement<G>
where
    G: Settlements,
{
    game: UpdateSender<G>,
}

impl<G> Processor for SettlementRefToSettlement<G>
where
    G: Settlements,
{
    fn process(&mut self, state: State, instruction: &Instruction) -> State {
        block_on(self.process(state, instruction))
    }
}

impl<G> SettlementRefToSettlement<G>
where
    G: Settlements,
{
    pub fn new(game: &UpdateSender<G>) -> SettlementRefToSettlement<G> {
        SettlementRefToSettlement {
            game: game.clone_with_handle(HANDLE),
        }
    }

    pub async fn process(&mut self, mut state: State, instruction: &Instruction) -> State {
        let position = match instruction {
            Instruction::SettlementRef(position) => *position,
            _ => return state,
        };
        let settlement = self.get_settlement(position).await;
        if let Some(settlement) = settlement {
            state.instructions.push(Instruction::Settlement(settlement));
        }
        state
    }

    async fn get_settlement(&mut self, position: V2<usize>) -> Option<Settlement> {
        self.game
            .update(move |settlements| get_settlement(settlements, position))
            .await
    }
}

fn get_settlement(settlements: &dyn Settlements, position: V2<usize>) -> Option<Settlement> {
    settlements.get_settlement(&position).cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    use commons::update::UpdateProcess;
    use commons::v2;
    use std::collections::HashMap;

    #[test]
    fn should_add_settlement_instruction_if_position_is_valid() {
        // Given
        let mut settlements = HashMap::new();
        settlements.insert(v2(1, 1), Settlement::default());
        let game = UpdateProcess::new(settlements);
        let mut processor = SettlementRefToSettlement::new(&game.tx());

        // When
        let state = block_on(async {
            processor
                .process(State::default(), &Instruction::SettlementRef(v2(1, 1)))
                .await
        });

        // Then
        assert_eq!(
            state.instructions[0],
            Instruction::Settlement(Settlement::default()),
        );

        // Finally
        game.shutdown();
    }

    #[test]
    fn should_add_no_instruction_if_position_is_invalid() {
        // Given
        let settlements = HashMap::new();
        let game = UpdateProcess::new(settlements);
        let mut processor = SettlementRefToSettlement::new(&game.tx());

        // When
        let state = block_on(async {
            processor
                .process(State::default(), &Instruction::SettlementRef(v2(1, 1)))
                .await
        });

        // Then
        assert!(state.instructions.is_empty());

        // Finally
        game.shutdown();
    }
}
