use super::*;

use crate::settlement::Settlement;
use crate::traits::{AddTown, WhoControlsTile};
use crate::update_territory::UpdateTerritory;
use commons::async_trait::async_trait;

pub struct TownBuilder<X, T>
where
    X: AddTown + WhoControlsTile + Send,
    T: UpdateTerritory,
{
    x: X,
    territory: T,
}

#[async_trait]
impl<X, T> Builder for TownBuilder<X, T>
where
    X: AddTown + WhoControlsTile + Send + Sync,
    T: UpdateTerritory + Send,
{
    fn can_build(&self, build: &Build) -> bool {
        if let Build::Town { .. } = build {
            true
        } else {
            false
        }
    }

    async fn build(&mut self, build: Build) {
        if let Build::Town(town) = build {
            let position = town.position;
            if self.try_add_town(town).await {
                self.territory.update_territory(position).await;
            }
        }
    }
}

impl<X, T> TownBuilder<X, T>
where
    X: AddTown + WhoControlsTile + Send + Sync,
    T: UpdateTerritory,
{
    pub fn new(x: X, territory: &T) -> TownBuilder<X, T> {
        TownBuilder {
            x,
            territory: territory.clone(),
        }
    }

    async fn try_add_town(&mut self, town: Settlement) -> bool {
        if self.x.who_controls_tile(&town.position).await.is_some() {
            return false;
        }
        self.x.add_town(town).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use commons::futures::executor::block_on;
    use commons::{v2, Arm, V2};
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    struct X {
        towns: Arm<HashMap<V2<usize>, Settlement>>,
        add_town_return: bool,
        control: HashMap<V2<usize>, V2<usize>>,
    }

    #[async_trait]
    impl AddTown for X {
        async fn add_town(&self, town: Settlement) -> bool {
            self.towns.lock().unwrap().insert(town.position, town);
            self.add_town_return
        }
    }

    #[async_trait]
    impl WhoControlsTile for X {
        async fn who_controls_tile(&self, position: &V2<usize>) -> Option<V2<usize>> {
            self.control.get(position).cloned()
        }
    }

    fn update_territory() -> Arc<Mutex<Vec<V2<usize>>>> {
        Arc::new(Mutex::new(vec![]))
    }

    #[test]
    fn can_build_town() {
        // Given
        let x = X::default();
        let builder = TownBuilder::new(x, &update_territory());

        // When
        let can_build = builder.can_build(&Build::Town(Settlement::default()));

        // Then
        assert!(can_build);
    }

    #[test]
    fn should_build_if_position_not_controlled() {
        // Given
        let town = Settlement {
            position: v2(1, 2),
            ..Settlement::default()
        };
        let x = X::default();
        let mut builder = TownBuilder::new(x, &update_territory());

        // When
        block_on(builder.build(Build::Town(town.clone())));

        // Then
        assert_eq!(
            *builder.x.towns.lock().unwrap(),
            hashmap! {town.position => town},
        );
    }

    #[test]
    fn should_not_build_if_position_controlled() {
        // Given
        let town = Settlement {
            position: v2(1, 2),
            ..Settlement::default()
        };
        let control = hashmap! { v2(1, 2) => v2(0, 0) };
        let x = X {
            control,
            ..X::default()
        };
        let mut builder = TownBuilder::new(x, &update_territory());

        // When
        block_on(builder.build(Build::Town(town)));

        // Then
        assert_eq!(*builder.x.towns.lock().unwrap(), hashmap! {},);
    }

    #[test]
    fn should_update_territory_if_settlement_built() {
        // Given
        let town = Settlement {
            position: v2(1, 2),
            ..Settlement::default()
        };
        let x = X {
            add_town_return: true,
            ..X::default()
        };
        let update_territory = update_territory();
        let mut builder = TownBuilder::new(x, &update_territory);

        // When
        block_on(builder.build(Build::Town(town)));

        // Then
        assert_eq!(*update_territory.lock().unwrap(), vec![v2(1, 2)]);
    }

    #[test]
    fn should_not_update_territory_if_settlement_not_built() {
        // Given
        let town = Settlement {
            position: v2(1, 2),
            ..Settlement::default()
        };
        let x = X {
            add_town_return: false,
            ..X::default()
        };
        let update_territory = update_territory();
        let mut builder = TownBuilder::new(x, &update_territory);

        // When
        block_on(builder.build(Build::Town(town)));

        // Then
        assert_eq!(*update_territory.lock().unwrap(), vec![]);
    }

    #[test]
    fn should_not_update_territory_if_position_controlled() {
        // Given
        let town = Settlement {
            position: v2(1, 2),
            ..Settlement::default()
        };
        let control = hashmap! { v2(1, 2) => v2(0, 0) };
        let x = X {
            control,
            ..X::default()
        };
        let update_territory = update_territory();
        let mut builder = TownBuilder::new(x, &update_territory);

        // When
        block_on(builder.build(Build::Town(town)));

        // Then
        assert_eq!(*update_territory.lock().unwrap(), vec![]);
    }
}
