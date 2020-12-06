use super::*;
use crate::game::traits::{Controlled, Settlements};
use crate::settlement::{Settlement, SettlementClass::Town};
use crate::traits::UpdateTerritory;
use std::collections::HashSet;

const NAME: &str = "get_territory";

pub struct GetTerritory<G, X>
where
    G: Send,
{
    game: FnSender<G>,
    x: X,
}

#[async_trait]
impl<G, X> Processor for GetTerritory<G, X>
where
    G: Controlled + Settlements + Send,
    X: UpdateTerritory + Send,
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

        self.x.update_territory(settlement.position).await;
        let territory = self.territory(settlement.position).await;

        state.instructions.push(Instruction::GetTownTraffic {
            settlement,
            territory,
        });

        state
    }
}

impl<G, X> GetTerritory<G, X>
where
    G: Controlled + Settlements + Send,
    X: UpdateTerritory,
{
    pub fn new(game: &FnSender<G>, x: X) -> GetTerritory<G, X> {
        GetTerritory {
            game: game.clone_with_name(NAME),
            x,
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
    use commons::{v2, Arm};
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

    #[async_trait]
    impl UpdateTerritory for Arm<Vec<V2<usize>>> {
        async fn update_territory(&mut self, controller: V2<usize>) {
            self.lock().unwrap().push(controller);
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

        let mut processor = GetTerritory::new(&game.tx(), updated_territory);

        // Given
        let instruction = Instruction::GetTerritory(settlement.position);
        let state = block_on(processor.process(State::default(), &instruction));

        // Then
        assert_eq!(*processor.x.lock().unwrap(), vec![settlement.position]);
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

        let mut processor = GetTerritory::new(&game.tx(), updated_territory);

        // Given
        let instruction = Instruction::GetTerritory(settlement.position);
        let state = block_on(processor.process(State::default(), &instruction));

        // Then
        assert_eq!(*processor.x.lock().unwrap(), vec![]);
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

        let mut processor = GetTerritory::new(&game.tx(), updated_territory);

        // Given
        let instruction = Instruction::GetTerritory(v2(5, 6));
        let state = block_on(processor.process(State::default(), &instruction));

        // Then
        assert_eq!(*processor.x.lock().unwrap(), vec![]);
        assert_eq!(state.instructions, vec![]);

        // Finally
        game.join();
    }
}
