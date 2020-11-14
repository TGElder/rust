use super::*;

use crate::game::traits::{AddSettlement, WhoControlsTile};
use crate::settlement::Settlement;
use crate::update_territory::UpdateTerritory;

const HANDLE: &str = "settlement_builder";

pub struct SettlementBuilder<G, T>
where
    G: AddSettlement + WhoControlsTile + Send,
    T: UpdateTerritory,
{
    game: FnSender<G>,
    territory: T,
}

#[async_trait]
impl<G, T> Builder for SettlementBuilder<G, T>
where
    G: AddSettlement + WhoControlsTile + Send,
    T: UpdateTerritory + Send,
{
    fn can_build(&self, build: &Build) -> bool {
        if let Build::Settlement { .. } = build {
            true
        } else {
            false
        }
    }

    async fn build(&mut self, build: Build) {
        if let Build::Settlement(settlement) = build {
            let position = settlement.position;
            if self.try_add_settlement(settlement).await {
                self.territory.update_territory(position).await;
            }
        }
    }
}

impl<G, T> SettlementBuilder<G, T>
where
    G: AddSettlement + WhoControlsTile + Send,
    T: UpdateTerritory,
{
    pub fn new(game: &FnSender<G>, territory: &T) -> SettlementBuilder<G, T> {
        SettlementBuilder {
            game: game.clone_with_name(HANDLE),
            territory: territory.clone(),
        }
    }

    async fn try_add_settlement(&mut self, settlement: Settlement) -> bool {
        self.game
            .send(move |game| try_add_settlement(game, settlement))
            .await
    }
}

fn try_add_settlement<G>(game: &mut G, settlement: Settlement) -> bool
where
    G: AddSettlement + WhoControlsTile + Send,
{
    if game.who_controls_tile(&settlement.position).is_some() {
        return false;
    };
    game.add_settlement(settlement)
}

#[cfg(test)]
mod tests {
    use super::*;

    use commons::fn_sender::FnThread;
    use commons::futures::executor::block_on;
    use commons::{v2, V2};
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    struct Game {
        settlements: HashMap<V2<usize>, Settlement>,
        add_settlement_return: bool,
        control: HashMap<V2<usize>, V2<usize>>,
    }

    impl AddSettlement for Game {
        fn add_settlement(&mut self, settlement: Settlement) -> bool {
            self.settlements.insert(settlement.position, settlement);
            self.add_settlement_return
        }
    }

    impl WhoControlsTile for Game {
        fn who_controls_tile(&self, position: &V2<usize>) -> Option<&V2<usize>> {
            self.control.get(position)
        }
    }

    fn update_territory() -> Arc<Mutex<Vec<V2<usize>>>> {
        Arc::new(Mutex::new(vec![]))
    }

    #[test]
    fn can_build_settlement() {
        // Given
        let game = FnThread::new(Game::default());
        let builder = SettlementBuilder::new(&game.tx(), &update_territory());

        // When
        let can_build = builder.can_build(&Build::Settlement(Settlement::default()));

        // Then
        assert!(can_build);

        // Finally
        game.join();
    }

    #[test]
    fn should_build_if_position_not_controlled() {
        // Given
        let settlement = Settlement {
            position: v2(1, 2),
            ..Settlement::default()
        };
        let game = Game::default();
        let game = FnThread::new(game);
        let mut builder = SettlementBuilder::new(&game.tx(), &update_territory());

        // When
        block_on(builder.build(Build::Settlement(settlement.clone())));
        let game = game.join();

        // Then
        assert_eq!(
            game.settlements,
            hashmap! {settlement.position => settlement},
        );
    }

    #[test]
    fn should_not_build_if_position_controlled() {
        // Given
        let settlement = Settlement {
            position: v2(1, 2),
            ..Settlement::default()
        };
        let control = hashmap! { v2(1, 2) => v2(0, 0) };
        let game = Game {
            control,
            ..Game::default()
        };
        let game = FnThread::new(game);
        let mut builder = SettlementBuilder::new(&game.tx(), &update_territory());

        // When
        block_on(builder.build(Build::Settlement(settlement)));
        let game = game.join();

        // Then
        assert_eq!(game.settlements, hashmap! {},);
    }

    #[test]
    fn should_update_territory_if_settlement_built() {
        // Given
        let settlement = Settlement {
            position: v2(1, 2),
            ..Settlement::default()
        };
        let game = Game {
            add_settlement_return: true,
            ..Game::default()
        };
        let game = FnThread::new(game);
        let update_territory = update_territory();
        let mut builder = SettlementBuilder::new(&game.tx(), &update_territory);

        // When
        block_on(builder.build(Build::Settlement(settlement)));

        // Then
        assert_eq!(*update_territory.lock().unwrap(), vec![v2(1, 2)]);

        // Finally
        game.join();
    }

    #[test]
    fn should_not_update_territory_if_settlement_not_built() {
        // Given
        let settlement = Settlement {
            position: v2(1, 2),
            ..Settlement::default()
        };
        let game = Game {
            add_settlement_return: false,
            ..Game::default()
        };
        let game = FnThread::new(game);
        let update_territory = update_territory();
        let mut builder = SettlementBuilder::new(&game.tx(), &update_territory);

        // When
        block_on(builder.build(Build::Settlement(settlement)));

        // Then
        assert_eq!(*update_territory.lock().unwrap(), vec![]);

        // Finally
        game.join();
    }

    #[test]
    fn should_not_update_territory_if_position_controlled() {
        // Given
        let settlement = Settlement {
            position: v2(1, 2),
            ..Settlement::default()
        };
        let control = hashmap! { v2(1, 2) => v2(0, 0) };
        let game = Game {
            control,
            ..Game::default()
        };
        let game = FnThread::new(game);
        let update_territory = update_territory();
        let mut builder = SettlementBuilder::new(&game.tx(), &update_territory);

        // When
        block_on(builder.build(Build::Settlement(settlement)));

        // Then
        assert_eq!(*update_territory.lock().unwrap(), vec![]);

        // Finally
        game.join();
    }
}
