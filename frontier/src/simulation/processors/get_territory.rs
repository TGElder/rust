use super::*;
use crate::settlement::SettlementClass::Town;
use crate::traits::{Controlled, GetSettlement, UpdateTerritory};

pub struct GetTerritory<T> {
    tx: T,
}

#[async_trait]
impl<T> Processor for GetTerritory<T>
where
    T: Controlled + GetSettlement + UpdateTerritory + Send + Sync,
{
    async fn process(&mut self, mut state: State, instruction: &Instruction) -> State {
        let settlement = match instruction {
            Instruction::GetTerritory(settlement) => *settlement,
            _ => return state,
        };

        let settlement = unwrap_or!(self.tx.get_settlement(settlement).await, return state);
        if settlement.class != Town {
            return state;
        };

        self.tx.update_territory(settlement.position).await;
        let territory = self.tx.controlled(settlement.position).await;

        state.instructions.push(Instruction::GetTownTraffic {
            settlement,
            territory,
        });

        state
    }
}

impl<T> GetTerritory<T>
where
    T: Controlled + GetSettlement + UpdateTerritory + Send,
{
    pub fn new(tx: T) -> GetTerritory<T> {
        GetTerritory { tx }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::settlement::Settlement;
    use crate::settlement::SettlementClass::Homeland;
    use commons::{v2, Arm};
    use futures::executor::block_on;
    use std::collections::{HashMap, HashSet};

    struct Tx {
        territory: HashSet<V2<usize>>,
        settlements: HashMap<V2<usize>, Settlement>,
        updated_territory: Arm<Vec<V2<usize>>>,
    }

    #[async_trait]
    impl Controlled for Tx {
        async fn controlled(&self, _: V2<usize>) -> HashSet<V2<usize>> {
            self.territory.clone()
        }
    }

    #[async_trait]
    impl GetSettlement for Tx {
        async fn get_settlement(&self, position: V2<usize>) -> Option<Settlement> {
            self.settlements.get(&position).cloned()
        }
    }

    #[async_trait]
    impl UpdateTerritory for Tx {
        async fn update_territory(&mut self, controller: V2<usize>) {
            self.updated_territory.lock().unwrap().push(controller);
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
        let updated_territory = Arm::default();
        let tx = Tx {
            territory: territory.clone(),
            settlements,
            updated_territory: updated_territory.clone(),
        };

        let mut processor = GetTerritory::new(tx);

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
        let updated_territory = Arm::default();
        let tx = Tx {
            territory,
            settlements,
            updated_territory: updated_territory.clone(),
        };

        let mut processor = GetTerritory::new(tx);

        // Given
        let instruction = Instruction::GetTerritory(settlement.position);
        let state = block_on(processor.process(State::default(), &instruction));

        // Then
        assert!(updated_territory.lock().unwrap().is_empty());
        assert_eq!(state.instructions, vec![]);
    }

    #[test]
    fn should_do_nothing_if_settlement_does_not_exist() {
        let territory = hashset! {};
        let settlements = hashmap! {};
        let updated_territory = Arm::default();
        let tx = Tx {
            territory,
            settlements,
            updated_territory: updated_territory.clone(),
        };

        let mut processor = GetTerritory::new(tx);

        // Given
        let instruction = Instruction::GetTerritory(v2(5, 6));
        let state = block_on(processor.process(State::default(), &instruction));

        // Then
        assert!(updated_territory.lock().unwrap().is_empty());
        assert_eq!(state.instructions, vec![]);
    }
}
