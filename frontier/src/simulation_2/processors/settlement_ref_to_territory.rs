use super::*;
use crate::game::traits::{Controlled, Settlements};
use crate::settlement::{Settlement, SettlementClass::Town};
use crate::update_territory::UpdateTerritory;
use std::collections::HashSet;

const HANDLE: &str = "settlement_ref_to_territory";

pub struct SettlementRefToTerritory<G, T>
where
    G: Controlled + Settlements,
    T: UpdateTerritory,
{
    game: UpdateSender<G>,
    territory: T,
}

impl<G, T> Processor for SettlementRefToTerritory<G, T>
where
    G: Controlled + Settlements,
    T: UpdateTerritory,
{
    fn process(&mut self, state: State, instruction: &Instruction) -> State {
        self.process(state, instruction)
    }
}

impl<G, T> SettlementRefToTerritory<G, T>
where
    G: Controlled + Settlements,
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

        let settlement = unwrap_or!(self.settlement(settlement), return state);
        if settlement.class != Town {
            return state;
        };

        self.territory.update_territory(settlement.position);
        let territory = self.territory(settlement.position);

        state.instructions.push(Instruction::Territory {
            settlement,
            territory,
        });

        state
    }

    fn settlement(&mut self, position: V2<usize>) -> Option<Settlement> {
        block_on(async {
            self.game
                .update(move |game| settlement(game, position))
                .await
        })
    }

    fn territory(&mut self, settlement: V2<usize>) -> HashSet<V2<usize>> {
        block_on(async {
            self.game
                .update(move |game| territory(game, settlement))
                .await
        })
    }
}

fn settlement<G>(game: &mut G, settlement: V2<usize>) -> Option<Settlement>
where
    G: Settlements,
{
    game.get_settlement(&settlement).cloned()
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

    use crate::settlement::SettlementClass::Homeland;
    use commons::update::UpdateProcess;
    use commons::v2;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    struct MockGame {
        territory: HashSet<V2<usize>>,
        settlements: HashMap<V2<usize>, Settlement>,
    }

    impl Controlled for MockGame {
        fn controlled(&self, position: &V2<usize>) -> HashSet<V2<usize>> {
            self.territory.clone()
        }
    }

    impl Settlements for MockGame {
        fn settlements(&self) -> &HashMap<V2<usize>, Settlement> {
            &self.settlements
        }
    }

    #[test]
    fn should_call_update_territory_and_return_controlled_if_settlement_class_is_town() {
        // When
        let settlement = Settlement {
            class: Town,
            position: v2(5, 6),
            ..Settlement::default()
        };
        let territory = hashset! { v2(1, 2), v2(3, 4) };
        let settlements = hashmap! {settlement.position => settlement.clone() };
        let game = MockGame {
            territory: territory.clone(),
            settlements,
        };
        let game = UpdateProcess::new(game);

        let updated_territory = Arc::new(Mutex::new(vec![]));

        let mut processor = SettlementRefToTerritory::new(&game.tx(), &updated_territory);

        // Given
        let instruction = Instruction::SettlementRef(settlement.position);
        let state = processor.process(State::default(), &instruction);

        // Then
        assert_eq!(
            *updated_territory.lock().unwrap(),
            vec![settlement.position]
        );
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

    #[test]
    fn should_do_nothing_if_settlement_class_not_town() {
        let settlement = Settlement {
            class: Homeland,
            position: v2(5, 6),
            ..Settlement::default()
        };
        let territory = hashset! {};
        let settlements = hashmap! {settlement.position => settlement.clone() };
        let game = MockGame {
            territory,
            settlements,
        };
        let game = UpdateProcess::new(game);

        let updated_territory = Arc::new(Mutex::new(vec![]));

        let mut processor = SettlementRefToTerritory::new(&game.tx(), &updated_territory);

        // Given
        let instruction = Instruction::SettlementRef(settlement.position);
        let state = processor.process(State::default(), &instruction);

        // Then
        assert_eq!(*updated_territory.lock().unwrap(), vec![]);
        assert_eq!(state.instructions, vec![]);

        // Finally
        game.shutdown();
    }

    #[test]
    fn should_do_nothing_if_settlement_does_not_exist() {
        let territory = hashset! {};
        let settlements = hashmap! {};
        let game = MockGame {
            territory,
            settlements,
        };
        let game = UpdateProcess::new(game);

        let updated_territory = Arc::new(Mutex::new(vec![]));

        let mut processor = SettlementRefToTerritory::new(&game.tx(), &updated_territory);

        // Given
        let instruction = Instruction::SettlementRef(v2(5, 6));
        let state = processor.process(State::default(), &instruction);

        // Then
        assert_eq!(*updated_territory.lock().unwrap(), vec![]);
        assert_eq!(state.instructions, vec![]);

        // Finally
        game.shutdown();
    }
}
