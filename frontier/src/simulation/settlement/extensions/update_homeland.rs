use crate::settlement::Settlement;
use crate::simulation::settlement::SettlementSimulation;
use crate::traits::has::HasParameters;
use crate::traits::VisibleLandPositions;

impl<T, D> SettlementSimulation<T, D>
where
    T: HasParameters + VisibleLandPositions,
{
    pub async fn update_homeland(&self, settlement: Settlement) -> Settlement {
        let visible_land_positions = self.cx.visible_land_positions().await;
        let homeland_count = self.cx.parameters().homeland.count;
        let target_population = visible_land_positions as f64 / homeland_count as f64;
        Settlement {
            target_population,
            ..settlement
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::parameters::Parameters;
    use crate::settlement::SettlementClass::Homeland;
    use commons::async_trait::async_trait;
    use commons::v2;
    use futures::executor::block_on;

    struct Cx {
        parameters: Parameters,
        visible_land_positions: usize,
    }

    impl HasParameters for Cx {
        fn parameters(&self) -> &crate::parameters::Parameters {
            &self.parameters
        }
    }

    #[async_trait]
    impl VisibleLandPositions for Cx {
        async fn visible_land_positions(&self) -> usize {
            self.visible_land_positions
        }
    }

    #[test]
    fn target_population_should_be_equal_share_of_visible_land() {
        // Given
        let mut cx = Cx {
            parameters: Parameters::default(),
            visible_land_positions: 202,
        };
        cx.parameters.homeland.count = 2;
        let sim = SettlementSimulation::new(cx, Arc::new(()));

        // When
        let settlement = Settlement {
            position: v2(0, 1),
            class: Homeland,
            ..Settlement::default()
        };
        let updated = block_on(sim.update_homeland(settlement));

        // Then
        assert_eq!(
            updated,
            Settlement {
                position: v2(0, 1),
                class: Homeland,
                target_population: 101.0,
                ..Settlement::default()
            }
        );
    }
}
