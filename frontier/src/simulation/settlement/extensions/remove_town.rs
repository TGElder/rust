use crate::settlement::Settlement;
use crate::simulation::settlement::model::TownTrafficSummary;
use crate::simulation::settlement::SettlementSimulation;
use crate::traits::has::HasParameters;
use crate::traits::{Controlled, RefreshPositions, RemoveTown as RemoveTownTrait};

impl<T, D> SettlementSimulation<T, D>
where
    T: Controlled + HasParameters + RefreshPositions + RemoveTownTrait,
{
    pub async fn remove_town(
        &self,
        settlement: &Settlement,
        traffic: &[TownTrafficSummary],
    ) -> bool {
        let town_removal_population = self.cx.parameters().simulation.town_removal_population;
        if settlement.current_population >= town_removal_population || !traffic.is_empty() {
            return false;
        }
        let controlled = self.cx.controlled(&settlement.position).await;
        self.cx.remove_town(&settlement.position).await;
        self.cx.refresh_positions(controlled).await;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::parameters::Parameters;
    use crate::settlement::Settlement;
    use commons::async_trait::async_trait;
    use commons::{v2, Arm, V2};
    use futures::executor::block_on;
    use std::collections::HashSet;
    use std::default::Default;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    #[derive(Default)]
    struct Cx {
        controlled: HashSet<V2<usize>>,
        parameters: Parameters,
        refreshed_positions: Mutex<HashSet<V2<usize>>>,
        removed: Arm<Vec<V2<usize>>>,
    }

    #[async_trait]
    impl Controlled for Cx {
        async fn controlled(&self, _: &V2<usize>) -> HashSet<V2<usize>> {
            self.controlled.clone()
        }
    }

    impl HasParameters for Cx {
        fn parameters(&self) -> &Parameters {
            &self.parameters
        }
    }

    #[async_trait]
    impl RefreshPositions for Cx {
        async fn refresh_positions(&self, positions: HashSet<V2<usize>>) {
            self.refreshed_positions.lock().unwrap().extend(positions);
        }
    }

    #[async_trait]
    impl RemoveTownTrait for Cx {
        async fn remove_town(&self, position: &V2<usize>) -> bool {
            self.removed.lock().unwrap().push(*position);
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
        let mut cx = Cx::default();
        cx.parameters.simulation.town_removal_population = 0.3;
        let sim = SettlementSimulation::new(cx, Arc::new(()));

        // When
        let removed = block_on(sim.remove_town(&settlement, &[]));

        // Then
        assert!(removed);
        assert_eq!(*sim.cx.removed.lock().unwrap(), vec![settlement.position]);
    }

    #[test]
    fn should_not_remove_town_with_current_population_below_threshold_but_traffic() {
        // Given
        let settlement = Settlement {
            current_population: 0.2,
            ..Settlement::default()
        };
        let mut cx = Cx::default();
        cx.parameters.simulation.town_removal_population = 0.3;
        let sim = SettlementSimulation::new(cx, Arc::new(()));

        // When
        let removed = block_on(sim.remove_town(
            &settlement,
            &[TownTrafficSummary {
                nation: "A".to_string(),
                traffic_share: 1.0,
                total_duration: Duration::default(),
            }],
        ));

        // Then
        assert!(!removed);
        assert!(sim.cx.removed.lock().unwrap().is_empty());
    }

    #[test]
    fn should_not_remove_town_with_no_traffic_but_current_population_above_threshold() {
        // Given
        let settlement = Settlement {
            current_population: 0.7,
            ..Settlement::default()
        };
        let mut cx = Cx::default();
        cx.parameters.simulation.town_removal_population = 0.3;
        let sim = SettlementSimulation::new(cx, Arc::new(()));

        // When
        let removed = block_on(sim.remove_town(&settlement, &[]));

        // Then
        assert!(!removed);
        assert!(sim.cx.removed.lock().unwrap().is_empty());
    }

    #[test]
    fn should_refresh_all_positions_controlled_by_town() {
        // Given
        let settlement = Settlement {
            current_population: 0.2,
            ..Settlement::default()
        };
        let mut cx = Cx {
            controlled: hashset! { v2(1, 2), v2(3, 4) },
            ..Cx::default()
        };
        cx.parameters.simulation.town_removal_population = 0.3;
        let sim = SettlementSimulation::new(cx, Arc::new(()));

        // When
        block_on(sim.remove_town(&settlement, &[]));

        assert_eq!(
            *sim.cx.refreshed_positions.lock().unwrap(),
            hashset! { v2(1, 2), v2(3, 4) },
        );
    }
}
