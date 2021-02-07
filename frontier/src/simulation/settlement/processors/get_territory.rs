use std::collections::HashSet;

use super::*;
use crate::traits::{Controlled, UpdateTerritory};

pub struct GetTerritory<T> {
    tx: T,
}

impl<T> GetTerritory<T>
where
    T: Controlled + UpdateTerritory,
{
    pub fn new(tx: T) -> GetTerritory<T> {
        GetTerritory { tx }
    }

    pub async fn get_territory(&self, controller: &V2<usize>) -> HashSet<V2<usize>> {
        self.tx.update_territory(*controller).await;
        self.tx.controlled(*controller).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use commons::{v2, Arm};
    use futures::executor::block_on;
    use std::collections::HashSet;

    struct Tx {
        territory: HashSet<V2<usize>>,
        updated_territory: Arm<Vec<V2<usize>>>,
    }

    #[async_trait]
    impl Controlled for Tx {
        async fn controlled(&self, _: V2<usize>) -> HashSet<V2<usize>> {
            self.territory.clone()
        }
    }

    #[async_trait]
    impl UpdateTerritory for Tx {
        async fn update_territory(&self, controller: V2<usize>) {
            self.updated_territory.lock().unwrap().push(controller);
        }
    }

    #[test]
    fn should_call_update_territory_and_return_controlled() {
        // When
        let territory = hashset! { v2(1, 2), v2(3, 4) };
        let updated_territory = Arm::default();
        let tx = Tx {
            territory: territory.clone(),
            updated_territory: updated_territory.clone(),
        };

        let get_territory = GetTerritory::new(tx);

        // Given
        let actual = block_on(get_territory.get_territory(&v2(0, 0)));

        // Then
        assert_eq!(*updated_territory.lock().unwrap(), vec![v2(0, 0)]);
        assert_eq!(actual, territory);
    }
}
