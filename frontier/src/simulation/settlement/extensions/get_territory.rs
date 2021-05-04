use std::collections::HashSet;

use commons::V2;

use crate::simulation::settlement::SettlementSimulation;
use crate::traits::{Controlled, UpdateTerritory};

impl<T, D> SettlementSimulation<T, D>
where
    T: Controlled + UpdateTerritory,
{
    pub async fn get_territory(&self, controller: &V2<usize>) -> HashSet<V2<usize>> {
        self.cx.update_territory(*controller).await;
        self.cx.controlled(controller).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use commons::async_trait::async_trait;
    use commons::{v2, Arm};
    use futures::executor::block_on;
    use std::collections::HashSet;
    use std::sync::Arc;

    struct Cx {
        territory: HashSet<V2<usize>>,
        updated_territory: Arm<Vec<V2<usize>>>,
    }

    #[async_trait]
    impl Controlled for Cx {
        async fn controlled(&self, _: &V2<usize>) -> HashSet<V2<usize>> {
            self.territory.clone()
        }
    }

    #[async_trait]
    impl UpdateTerritory for Cx {
        async fn update_territory(&self, controller: V2<usize>) {
            self.updated_territory.lock().unwrap().push(controller);
        }
    }

    #[test]
    fn should_call_update_territory_and_return_controlled() {
        // When
        let territory = hashset! { v2(1, 2), v2(3, 4) };
        let updated_territory = Arm::default();
        let cx = Cx {
            territory: territory.clone(),
            updated_territory: updated_territory.clone(),
        };

        let sim = SettlementSimulation::new(cx, Arc::new(()));

        // Given
        let actual = block_on(sim.get_territory(&v2(0, 0)));

        // Then
        assert_eq!(*updated_territory.lock().unwrap(), vec![v2(0, 0)]);
        assert_eq!(actual, territory);
    }
}
