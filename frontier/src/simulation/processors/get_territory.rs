use super::*;
use crate::game::traits::{Controlled, Settlements};
use crate::settlement::{Settlement, SettlementClass::Town};
use crate::update_territory::UpdateTerritory;
use std::collections::HashSet;

const NAME: &str = "get_territory";

pub struct GetTerritory<G, T>
where
    G: Controlled + Settlements + Send,
    T: UpdateTerritory,
{
    game: FnSender<G>,
    territory: T,
}

#[async_trait]
impl<G, T> Processor for GetTerritory<G, T>
where
    G: Controlled + Settlements + Send,
    T: UpdateTerritory + Send,
{
    async fn process(&mut self, mut state: State, instruction: &Instruction) -> State {
        let settlement = match instruction {
            Instruction::GetTerritory(settlement) => *settlement,
            _ => return state,
        };

        let settlement = unwrap_or!(self.settlement(settlement).await, return state);
        if settlement.class != Town {
            return state;
        };

        self.territory.update_territory(settlement.position).await;
        let territory = self.territory(settlement.position).await;

        state.instructions.push(Instruction::GetTownTraffic {
            settlement,
            territory,
        });

        state
    }
}

impl<G, T> GetTerritory<G, T>
where
    G: Controlled + Settlements + Send,
    T: UpdateTerritory,
{
    pub fn new(game: &FnSender<G>, territory: &T) -> GetTerritory<G, T> {
        GetTerritory {
            game: game.clone_with_name(NAME),
            territory: territory.clone(),
        }
    }

    async fn settlement(&mut self, position: V2<usize>) -> Option<Settlement> {
        self.game.send(move |game| settlement(game, position)).await
    }

    async fn territory(&mut self, settlement: V2<usize>) -> HashSet<V2<usize>> {
        self.game
            .send(move |game| territory(game, settlement))
            .await
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
    use commons::fn_sender::FnThread;
    use commons::futures::executor::block_on;
    use commons::v2;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    struct MockGame {
        territory: HashSet<V2<usize>>,
        settlements: HashMap<V2<usize>, Settlement>,
    }

    impl Controlled for MockGame {
        fn controlled(&self, _: &V2<usize>) -> HashSet<V2<usize>> {
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
        let game = FnThread::new(game);

        let updated_territory = Arc::new(Mutex::new(vec![]));

        let mut processor = GetTerritory::new(&game.tx(), &updated_territory);

        // Given
        let instruction = Instruction::GetTerritory(settlement.position);
        let state = block_on(processor.process(State::default(), &instruction));

        // Then
        assert_eq!(
            *updated_territory.lock().unwrap(),
            vec![settlement.position]
        );
        assert_eq!(
            state.instructions[0],
            Instruction::GetTownTraffic {
                settlement,
                territory
            }
        );

        // Finally
        game.join();
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
        let game = FnThread::new(game);

        let updated_territory = Arc::new(Mutex::new(vec![]));

        let mut processor = GetTerritory::new(&game.tx(), &updated_territory);

        // Given
        let instruction = Instruction::GetTerritory(settlement.position);
        let state = block_on(processor.process(State::default(), &instruction));

        // Then
        assert_eq!(*updated_territory.lock().unwrap(), vec![]);
        assert_eq!(state.instructions, vec![]);

        // Finally
        game.join();
    }

    #[test]
    fn should_do_nothing_if_settlement_does_not_exist() {
        let territory = hashset! {};
        let settlements = hashmap! {};
        let game = MockGame {
            territory,
            settlements,
        };
        let game = FnThread::new(game);

        let updated_territory = Arc::new(Mutex::new(vec![]));

        let mut processor = GetTerritory::new(&game.tx(), &updated_territory);

        // Given
        let instruction = Instruction::GetTerritory(v2(5, 6));
        let state = block_on(processor.process(State::default(), &instruction));

        // Then
        assert_eq!(*updated_territory.lock().unwrap(), vec![]);
        assert_eq!(state.instructions, vec![]);

        // Finally
        game.join();
    }
}
