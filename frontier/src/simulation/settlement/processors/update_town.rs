use std::time::Duration;

use super::*;
use crate::settlement::Settlement;
use crate::traits::UpdateSettlement;
use commons::unsafe_ordering;

pub struct UpdateTown<T> {
    tx: T,
    traffic_to_population: f64,
    nation_flip_traffic_pc: f64,
}

impl<T> UpdateTown<T>
where
    T: UpdateSettlement,
{
    pub fn new(tx: T, traffic_to_population: f64, nation_flip_traffic_pc: f64) -> UpdateTown<T> {
        UpdateTown {
            tx,
            traffic_to_population,
            nation_flip_traffic_pc,
        }
    }

    pub async fn update_town(&self, settlement: &Settlement, traffic: &[TownTrafficSummary]) {
        self.tx
            .update_settlement(Settlement {
                target_population: get_target_population(traffic, self.traffic_to_population),
                nation: get_nation(&settlement.nation, traffic, self.nation_flip_traffic_pc),
                gap_half_life: get_gap_half_life(settlement.gap_half_life, traffic),
                ..settlement.clone()
            })
            .await;
    }
}

fn get_target_population(
    traffic_summaries: &[TownTrafficSummary],
    traffic_to_population: f64,
) -> f64 {
    let total_traffic_share: f64 = traffic_summaries
        .iter()
        .map(|traffic_summary| traffic_summary.traffic_share)
        .sum();
    total_traffic_share * traffic_to_population
}

fn get_nation(
    original_nation: &str,
    traffic_summaries: &[TownTrafficSummary],
    nation_flip_traffic_pc: f64,
) -> String {
    let total_traffic_share: f64 = traffic_summaries
        .iter()
        .map(|traffic_summary| traffic_summary.traffic_share)
        .sum();
    if total_traffic_share == 0.0 {
        return original_nation.to_string();
    }
    let traffic_summary = traffic_summaries
        .iter()
        .max_by(|a, b| unsafe_ordering(&a.traffic_share, &b.traffic_share))
        .unwrap();
    if traffic_summary.traffic_share / total_traffic_share >= nation_flip_traffic_pc {
        traffic_summary.nation.clone()
    } else {
        original_nation.to_string()
    }
}

fn get_gap_half_life(original: Duration, traffic_summaries: &[TownTrafficSummary]) -> Duration {
    if traffic_summaries.is_empty() {
        return original;
    }
    let numerator = traffic_summaries
        .iter()
        .map(|summary| summary.total_duration)
        .sum::<Duration>()
        * 2;
    let denominator = traffic_summaries
        .iter()
        .map(|summary| summary.traffic_share)
        .sum::<f64>();
    numerator.div_f64(denominator).mul_f64(2.41) // converting "three-quarter life" to half life
}

#[cfg(test)]
mod tests {
    use super::*;

    use commons::almost::Almost;
    use commons::{v2, Arm};
    use futures::executor::block_on;

    use std::collections::HashMap;
    use std::default::Default;
    use std::sync::Mutex;

    #[async_trait]
    impl UpdateSettlement for Arm<HashMap<V2<usize>, Settlement>> {
        async fn update_settlement(&self, settlement: Settlement) {
            self.lock().unwrap().insert(settlement.position, settlement);
        }
    }

    #[test]
    fn should_update_target_population_based_on_total_traffic_share() {
        // Given
        let settlement = Settlement::default();
        let update_town = UpdateTown::new(Arc::new(Mutex::new(hashmap! {})), 0.5, 0.67);

        // When
        block_on(update_town.update_town(
            &settlement,
            &[
                TownTrafficSummary {
                    nation: "A".to_string(),
                    traffic_share: 17.0,
                    total_duration: Duration::default(),
                },
                TownTrafficSummary {
                    nation: "B".to_string(),
                    traffic_share: 39.0,
                    total_duration: Duration::default(),
                },
            ],
        ));

        // Then
        let updated_settlements = update_town.tx.lock().unwrap();
        assert!(updated_settlements[&v2(0, 0)]
            .target_population
            .almost(&28.0));
    }

    #[test]
    fn should_update_target_population_to_zero_for_town_with_no_traffic() {
        // Given
        let settlement = Settlement {
            target_population: 0.5,
            ..Settlement::default()
        };
        let update_town = UpdateTown::new(Arc::new(Mutex::new(hashmap! {})), 0.5, 0.67);

        // When
        block_on(update_town.update_town(&settlement, &[]));

        // Then
        let updated_settlements = update_town.tx.lock().unwrap();
        assert!(updated_settlements[&v2(0, 0)]
            .target_population
            .almost(&0.0));
    }

    #[test]
    fn should_update_town_nation_if_any_nation_exceeds_nation_flip_traffic_pc() {
        // Given
        let settlement = Settlement {
            nation: "A".to_string(),
            ..Settlement::default()
        };
        let update_town = UpdateTown::new(Arc::new(Mutex::new(hashmap! {})), 0.5, 0.67);

        // When
        block_on(update_town.update_town(
            &settlement,
            &[
                TownTrafficSummary {
                    nation: "B".to_string(),
                    traffic_share: 32.0,
                    total_duration: Duration::default(),
                },
                TownTrafficSummary {
                    nation: "C".to_string(),
                    traffic_share: 68.0,
                    total_duration: Duration::default(),
                },
            ],
        ));

        // Then
        let updated_settlements = update_town.tx.lock().unwrap();
        assert_eq!(updated_settlements[&v2(0, 0)].nation, "C".to_string(),);
    }

    #[test]
    fn should_keep_original_nation_if_no_nation_exceeds_nation_flip_traffic_pc() {
        // Given
        let settlement = Settlement {
            nation: "A".to_string(),
            ..Settlement::default()
        };
        let update_town = UpdateTown::new(Arc::new(Mutex::new(hashmap! {})), 0.5, 0.67);

        // When
        block_on(update_town.update_town(
            &settlement,
            &[
                TownTrafficSummary {
                    nation: "B".to_string(),
                    traffic_share: 40.0,
                    total_duration: Duration::default(),
                },
                TownTrafficSummary {
                    nation: "C".to_string(),
                    traffic_share: 60.0,
                    total_duration: Duration::default(),
                },
            ],
        ));

        // Then
        let updated_settlements = update_town.tx.lock().unwrap();
        assert_eq!(updated_settlements[&v2(0, 0)].nation, "A".to_string());
    }

    #[test]
    // equivalent half life is (ln(0.5) / ln(0.75)) * three-quarter life
    fn should_set_gap_three_quarter_life_to_total_round_trip_duration_divided_by_total_traffic_share(
    ) {
        // Given
        let settlement = Settlement::default();
        let update_town = UpdateTown::new(Arc::new(Mutex::new(hashmap! {})), 0.5, 0.67);

        // When
        block_on(update_town.update_town(
            &settlement,
            &[
                TownTrafficSummary {
                    nation: "A".to_string(),
                    traffic_share: 9.0,
                    total_duration: Duration::from_millis(9),
                },
                TownTrafficSummary {
                    nation: "B".to_string(),
                    traffic_share: 3.0,
                    total_duration: Duration::from_millis(27),
                },
            ],
        ));

        // Then
        let updated_settlements = update_town.tx.lock().unwrap();
        let gap_half_life_millis =
            updated_settlements[&v2(0, 0)].gap_half_life.as_nanos() as f32 / 1000000.0;
        assert!(gap_half_life_millis.almost(&14.46));
    }

    #[test]
    fn should_not_change_gap_half_life_for_town_with_no_traffic() {
        // Given
        let settlement = Settlement {
            target_population: 0.5,
            gap_half_life: Duration::from_millis(4),
            ..Settlement::default()
        };
        let update_town = UpdateTown::new(Arc::new(Mutex::new(hashmap! {})), 0.5, 0.67);

        // When
        block_on(update_town.update_town(&settlement, &[]));

        // Then
        let updated_settlements = update_town.tx.lock().unwrap();
        assert_eq!(
            updated_settlements[&v2(0, 0)].gap_half_life,
            Duration::from_millis(4)
        );
    }
}
