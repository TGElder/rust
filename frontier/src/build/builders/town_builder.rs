use super::*;

use crate::settlement::Settlement;
use crate::traits::{AddTown, UpdateTerritory, WhoControlsTile};
use commons::async_trait::async_trait;

pub struct TownBuilder<T> {
    cx: T,
}

#[async_trait]
impl<T> Builder for TownBuilder<T>
where
    T: AddTown + UpdateTerritory + WhoControlsTile + Send + Sync,
{
    fn can_build(&self, build: &Build) -> bool {
        matches!(build, Build::Town { .. })
    }

    async fn build(&mut self, build: Vec<Build>) {
        for build in build {
            self.try_build(build).await;
        }
    }
}

impl<T> TownBuilder<T>
where
    T: AddTown + UpdateTerritory + WhoControlsTile + Send + Sync,
{
    pub fn new(cx: T) -> TownBuilder<T> {
        TownBuilder { cx }
    }

    async fn try_build(&self, build: Build) {
        if let Build::Town(town) = build {
            let position = town.position;
            if self.try_add_town(town).await {
                self.cx.update_territory(position).await;
            }
        }
    }

    async fn try_add_town(&self, town: Settlement) -> bool {
        if self.cx.who_controls_tile(&town.position).await.is_some() {
            return false;
        }
        self.cx.add_town(town).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use commons::{v2, Arm, V2};
    use futures::executor::block_on;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    struct Cx {
        towns: Arm<HashMap<V2<usize>, Settlement>>,
        add_town_return: bool,
        control: HashMap<V2<usize>, V2<usize>>,
        updated_territory: Arc<Mutex<Vec<V2<usize>>>>,
    }

    #[async_trait]
    impl AddTown for Cx {
        async fn add_town(&self, town: Settlement) -> bool {
            self.towns.lock().unwrap().insert(town.position, town);
            self.add_town_return
        }
    }

    #[async_trait]
    impl UpdateTerritory for Cx {
        async fn update_territory(&self, controller: V2<usize>) {
            self.updated_territory.lock().unwrap().push(controller);
        }
    }

    #[async_trait]
    impl WhoControlsTile for Cx {
        async fn who_controls_tile(&self, position: &V2<usize>) -> Option<V2<usize>> {
            self.control.get(position).cloned()
        }
    }

    #[test]
    fn can_build_town() {
        // Given
        let cx = Cx::default();
        let builder = TownBuilder::new(cx);

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
        let cx = Cx::default();
        let mut builder = TownBuilder::new(cx);

        // When
        block_on(builder.build(vec![Build::Town(town.clone())]));

        // Then
        assert_eq!(
            *builder.cx.towns.lock().unwrap(),
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
        let cx = Cx {
            control,
            ..Cx::default()
        };
        let mut builder = TownBuilder::new(cx);

        // When
        block_on(builder.build(vec![Build::Town(town)]));

        // Then
        assert_eq!(*builder.cx.towns.lock().unwrap(), hashmap! {},);
    }

    #[test]
    fn should_update_territory_if_settlement_built() {
        // Given
        let town = Settlement {
            position: v2(1, 2),
            ..Settlement::default()
        };
        let cx = Cx {
            add_town_return: true,
            ..Cx::default()
        };
        let mut builder = TownBuilder::new(cx);

        // When
        block_on(builder.build(vec![Build::Town(town)]));

        // Then
        assert_eq!(
            *builder.cx.updated_territory.lock().unwrap(),
            vec![v2(1, 2)]
        );
    }

    #[test]
    fn should_not_update_territory_if_settlement_not_built() {
        // Given
        let town = Settlement {
            position: v2(1, 2),
            ..Settlement::default()
        };
        let cx = Cx {
            add_town_return: false,
            ..Cx::default()
        };
        let mut builder = TownBuilder::new(cx);

        // When
        block_on(builder.build(vec![Build::Town(town)]));

        // Then
        assert!(builder.cx.updated_territory.lock().unwrap().is_empty());
    }

    #[test]
    fn should_not_update_territory_if_position_controlled() {
        // Given
        let town = Settlement {
            position: v2(1, 2),
            ..Settlement::default()
        };
        let control = hashmap! { v2(1, 2) => v2(0, 0) };
        let cx = Cx {
            control,
            ..Cx::default()
        };
        let mut builder = TownBuilder::new(cx);

        // When
        block_on(builder.build(vec![Build::Town(town)]));

        // Then
        assert!(builder.cx.updated_territory.lock().unwrap().is_empty());
    }

    #[test]
    fn should_build_all_towns() {
        // Given
        let town_1 = Settlement {
            position: v2(1, 2),
            ..Settlement::default()
        };
        let town_2 = Settlement {
            position: v2(3, 4),
            ..Settlement::default()
        };
        let cx = Cx::default();
        let mut builder = TownBuilder::new(cx);

        // When
        block_on(builder.build(vec![
            Build::Town(town_1.clone()),
            Build::Town(town_2.clone()),
        ]));

        // Then
        assert_eq!(
            *builder.cx.towns.lock().unwrap(),
            hashmap! {
                town_1.position => town_1,
                town_2.position => town_2
            },
        );
    }
}
