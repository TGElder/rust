use super::*;
use crate::traits::{Controlled, RefreshBuildSim, RemoveTown as RemoveTownTrait};

pub struct RemoveTown<T> {
    tx: T,
    town_removal_population: f64,
}

#[async_trait]
impl<T> Processor for RemoveTown<T>
where
    T: Controlled + RefreshBuildSim + RemoveTownTrait + Send + Sync,
{
    async fn process(&mut self, state: State, instruction: &Instruction) -> State {
        let (settlement, traffic) = match instruction {
            Instruction::UpdateTown {
                settlement,
                traffic,
            } => (settlement, traffic),
            _ => return state,
        };
        if settlement.current_population >= self.town_removal_population || !traffic.is_empty() {
            return state;
        }
        let controlled = self.tx.controlled(settlement.position).await;
        self.tx.remove_town(settlement.position).await;
        self.tx.refresh_positions(controlled);
        state
    }
}

impl<T> RemoveTown<T>
where
    T: Controlled + RefreshBuildSim + RemoveTownTrait + Send,
{
    pub fn new(tx: T, town_removal_population: f64) -> RemoveTown<T> {
        RemoveTown {
            tx,
            town_removal_population,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::settlement::Settlement;
    use commons::{v2, Arm};
    use futures::executor::block_on;
    use std::collections::HashSet;
    use std::default::Default;
    use std::sync::Mutex;
    use std::time::Duration;

    #[derive(Default)]
    struct Tx {
        controlled: HashSet<V2<usize>>,
        refreshed_positions: Mutex<HashSet<V2<usize>>>,
        removed: Arm<Vec<V2<usize>>>,
    }

    #[async_trait]
    impl Controlled for Tx {
        async fn controlled(&self, _: V2<usize>) -> HashSet<V2<usize>> {
            self.controlled.clone()
        }
    }

    impl RefreshBuildSim for Tx {
        fn refresh_edges(&self, _: HashSet<commons::edge::Edge>) {}

        fn refresh_positions(&self, positions: HashSet<V2<usize>>) {
            self.refreshed_positions.lock().unwrap().extend(positions);
        }
    }

    #[async_trait]
    impl RemoveTownTrait for Tx {
        async fn remove_town(&self, position: V2<usize>) -> bool {
            self.removed.lock().unwrap().push(position);
            true
        }
    }

    #[test]
    fn should_remove_town_with_no_traffic_and_current_population_below_threshold() {
        // Given
        let settlement = Settlement {
            current_population: 0.2,
            ..Settlement::default()
        };
        let tx = Tx::default();
        let mut processor = RemoveTown::new(tx, 0.5);

        // When
        let instruction = Instruction::UpdateTown {
            settlement: settlement.clone(),
            traffic: vec![],
        };
        block_on(processor.process(State::default(), &instruction));

        // Then
        assert_eq!(
            *processor.tx.removed.lock().unwrap(),
            vec![settlement.position]
        );
    }

    #[test]
    fn should_not_remove_town_with_current_population_below_threshold_but_traffic() {
        // Given
        let settlement = Settlement {
            current_population: 0.2,
            ..Settlement::default()
        };
        let tx = Tx::default();
        let mut processor = RemoveTown::new(tx, 0.5);

        // When
        let instruction = Instruction::UpdateTown {
            settlement,
            traffic: vec![TownTrafficSummary {
                nation: "A".to_string(),
                traffic_share: 1.0,
                total_duration: Duration::default(),
            }],
        };
        block_on(processor.process(State::default(), &instruction));

        // Then
        assert!(processor.tx.removed.lock().unwrap().is_empty());
    }

    #[test]
    fn should_not_remove_town_with_no_traffic_but_current_population_above_threshold() {
        // Given
        let settlement = Settlement {
            current_population: 0.7,
            ..Settlement::default()
        };
        let tx = Tx::default();
        let mut processor = RemoveTown::new(tx, 0.5);

        // When
        let instruction = Instruction::UpdateTown {
            settlement,
            traffic: vec![],
        };
        block_on(processor.process(State::default(), &instruction));

        // Then
        assert!(processor.tx.removed.lock().unwrap().is_empty());
    }

    #[test]
    fn should_refresh_all_positions_controlled_by_town() {
        // Given
        let settlement = Settlement {
            current_population: 0.2,
            ..Settlement::default()
        };
        let tx = Tx {
            controlled: hashset! { v2(1, 2), v2(3, 4) },
            ..Tx::default()
        };
        let mut processor = RemoveTown::new(tx, 0.5);

        // When
        let instruction = Instruction::UpdateTown {
            settlement,
            traffic: vec![],
        };
        block_on(processor.process(State::default(), &instruction));

        assert_eq!(
            *processor.tx.refreshed_positions.lock().unwrap(),
            hashset! { v2(1, 2), v2(3, 4) },
        );
    }
}
