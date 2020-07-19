use super::*;
use crate::game::traits::Controlled;
use crate::update_territory::UpdateTerritory;
use std::collections::HashSet;

const HANDLE: &str = "settlement_ref_to_territory";

pub struct SettlementRefToTerritory<G, T>
where
    G: Controlled,
    T: UpdateTerritory,
{
    game: UpdateSender<G>,
    territory: T,
}

impl<G, T> Processor for SettlementRefToTerritory<G, T>
where
    G: Controlled,
    T: UpdateTerritory,
{
    fn process(&mut self, state: State, instruction: &Instruction) -> State {
        self.process(state, instruction)
    }
}

impl<G, T> SettlementRefToTerritory<G, T>
where
    G: Controlled,
    T: UpdateTerritory,
{
    pub fn new(game: &UpdateSender<G>, territory: &T) -> SettlementRefToTerritory<G, T> {
        SettlementRefToTerritory {
            game: game.clone_with_handle(HANDLE),
            territory: territory.clone(),
        }
    }

    pub fn process(&mut self, mut state: State, instruction: &Instruction) -> State {
        let settlement = match instruction {
            Instruction::SettlementRef(settlement) => *settlement,
            _ => return state,
        };
        self.territory.update_territory(settlement);
        let territory = self.territory(settlement);
        state.instructions.push(Instruction::Territory {
            settlement,
            territory,
        });
        state
    }

    fn territory(&mut self, settlement: V2<usize>) -> HashSet<V2<usize>> {
        block_on(async {
            self.game
                .update(move |game| territory(game, settlement))
                .await
        })
    }
}

fn territory<G>(game: &mut G, settlement: V2<usize>) -> HashSet<V2<usize>>
where
    G: Controlled,
{
    game.controlled(&settlement)
}

#[cfg(test)]
mod tests {
    use super::*;

    use commons::update::UpdateProcess;
    use commons::v2;
    use std::sync::{Arc, Mutex};

    #[test]
    fn should_call_update_territory_and_return_controlled() {
        // When
        let settlement = v2(5, 6);
        let territory = hashset! { v2(1, 2), v2(3, 4) };
        let game = UpdateProcess::new(territory.clone());

        let updated_territory = Arc::new(Mutex::new(vec![]));

        let mut processor = SettlementRefToTerritory::new(&game.tx(), &updated_territory);

        // Given
        let instruction = Instruction::SettlementRef(settlement);
        let state = processor.process(State::default(), &instruction);

        // Then
        assert_eq!(*updated_territory.lock().unwrap(), vec![settlement]);
        assert_eq!(
            state.instructions[0],
            Instruction::Territory {
                settlement,
                territory
            }
        );

        // Finally
        game.shutdown();
    }
}
