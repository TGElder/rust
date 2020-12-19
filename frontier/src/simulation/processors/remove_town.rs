use super::*;
use crate::traits::{Controlled, RemoveTown as RemoveTownTrait};

pub struct RemoveTown<X> {
    x: X,
}

#[async_trait]
impl<X> Processor for RemoveTown<X>
where
    X: Controlled + RemoveTownTrait + Send + Sync,
{
    async fn process(&mut self, mut state: State, instruction: &Instruction) -> State {
        let (settlement, traffic) = match instruction {
            Instruction::UpdateTown {
                settlement,
                traffic,
            } => (settlement, traffic),
            _ => return state,
        };
        if settlement.current_population >= state.params.town_removal_population
            || !traffic.is_empty()
        {
            return state;
        }
        let controlled = self.x.controlled(settlement.position).await;
        self.x.remove_town(settlement.position).await;
        state
            .instructions
            .push(Instruction::RefreshPositions(controlled));
        state
    }
}

impl<X> RemoveTown<X>
where
    X: Controlled + RemoveTownTrait + Send,
{
    pub fn new(x: X) -> RemoveTown<X> {
        RemoveTown { x }
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
    use std::time::Duration;

    #[derive(Default)]
    struct X {
        controlled: HashSet<V2<usize>>,
        removed: Arm<Vec<V2<usize>>>,
    }

    #[async_trait]
    impl Controlled for X {
        async fn controlled(&self, _: V2<usize>) -> HashSet<V2<usize>> {
            self.controlled.clone()
        }
    }

    #[async_trait]
    impl RemoveTownTrait for X {
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
        let x = X::default();
        let mut processor = RemoveTown::new(x);
        let state = State {
            params: SimulationParams {
                town_removal_population: 0.5,
                ..SimulationParams::default()
            },
            ..State::default()
        };

        // When
        let instruction = Instruction::UpdateTown {
            settlement: settlement.clone(),
            traffic: vec![],
        };
        block_on(processor.process(state, &instruction));

        // Then
        assert_eq!(
            *processor.x.removed.lock().unwrap(),
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
        let x = X::default();
        let mut processor = RemoveTown::new(x);
        let state = State {
            params: SimulationParams {
                town_removal_population: 0.5,
                ..SimulationParams::default()
            },
            ..State::default()
        };

        // When
        let instruction = Instruction::UpdateTown {
            settlement,
            traffic: vec![TownTrafficSummary {
                nation: "A".to_string(),
                traffic_share: 1.0,
                total_duration: Duration::default(),
            }],
        };
        block_on(processor.process(state, &instruction));

        // Then
        assert!(processor.x.removed.lock().unwrap().is_empty());
    }

    #[test]
    fn should_not_remove_town_with_no_traffic_but_current_population_above_threshold() {
        // Given
        let settlement = Settlement {
            current_population: 0.7,
            ..Settlement::default()
        };
        let x = X::default();
        let mut processor = RemoveTown::new(x);
        let state = State {
            params: SimulationParams {
                town_removal_population: 0.5,
                ..SimulationParams::default()
            },
            ..State::default()
        };

        // When
        let instruction = Instruction::UpdateTown {
            settlement,
            traffic: vec![],
        };
        block_on(processor.process(state, &instruction));

        // Then
        assert!(processor.x.removed.lock().unwrap().is_empty());
    }

    #[test]
    fn should_refresh_all_positions_controlled_by_town() {
        // Given
        let settlement = Settlement {
            current_population: 0.2,
            ..Settlement::default()
        };
        let x = X {
            controlled: hashset! { v2(1, 2), v2(3, 4) },
            ..X::default()
        };
        let mut processor = RemoveTown::new(x);
        let state = State {
            params: SimulationParams {
                town_removal_population: 0.5,
                ..SimulationParams::default()
            },
            ..State::default()
        };

        // When
        let instruction = Instruction::UpdateTown {
            settlement,
            traffic: vec![],
        };
        let state = block_on(processor.process(state, &instruction));

        assert_eq!(
            state.instructions,
            vec![Instruction::RefreshPositions(
                hashset! { v2(1, 2), v2(3, 4) },
            )]
        );
    }
}
