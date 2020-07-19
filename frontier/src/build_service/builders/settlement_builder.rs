use super::*;

use crate::game::traits::{AddSettlement, WhoControlsTile};
use crate::settlement::Settlement;
use crate::update_territory::UpdateTerritory;
use commons::V2;

const HANDLE: &str = "settlemen_builder";

pub struct SettlementBuilder<G, T>
where
    G: AddSettlement + WhoControlsTile,
    T: UpdateTerritory,
{
    game: UpdateSender<G>,
    territory: T,
}

impl<G, T> Builder for SettlementBuilder<G, T>
where
    G: AddSettlement + WhoControlsTile,
    T: UpdateTerritory,
{
    fn can_build(&self, build: &Build) -> bool {
        if let Build::Settlement { .. } = build {
            true
        } else {
            false
        }
    }

    fn build(&mut self, build: Build) {
        if let Build::Settlement {
            candidate_positions,
            mut settlement,
        } = build
        {
            for position in candidate_positions {
                if self.is_controlled(position) {
                    continue;
                }
                settlement.position = position;
                if self.add_settlement(settlement) {
                    self.territory.update_territory(position);
                }
                return;
            }
        }
    }
}

impl<G, T> SettlementBuilder<G, T>
where
    G: AddSettlement + WhoControlsTile,
    T: UpdateTerritory,
{
    pub fn new(game: &UpdateSender<G>, territory: &T) -> SettlementBuilder<G, T> {
        SettlementBuilder {
            game: game.clone_with_handle(HANDLE),
            territory: territory.clone(),
        }
    }

    fn is_controlled(&mut self, position: V2<usize>) -> bool {
        block_on(async {
            self.game
                .update(move |game| game.who_controls_tile(&position).is_some())
                .await
        })
    }

    fn add_settlement(&mut self, settlement: Settlement) -> bool {
        block_on(async {
            self.game
                .update(move |game| game.add_settlement(settlement))
                .await
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use commons::update::UpdateProcess;
    use commons::v2;
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
        let game = UpdateProcess::new(Game::default());
        let builder = SettlementBuilder::new(&game.tx(), &update_territory());

        // When
        let can_build = builder.can_build(&Build::Settlement {
            candidate_positions: vec![],
            settlement: Settlement::default(),
        });

        // Then
        assert!(can_build);

        // Finally
        game.shutdown();
    }

    #[test]
    fn should_build_at_first_position_not_controlled() {
        // Given
        let candidate_positions = vec![v2(1, 1), v2(1, 2), v2(2, 1)];
        let settlement = Settlement {
            position: candidate_positions[1],
            ..Settlement::default()
        };
        let control = vec![(candidate_positions[0], v2(0, 0))]
            .into_iter()
            .collect();
        let game = Game {
            control,
            ..Game::default()
        };
        let game = UpdateProcess::new(game);
        let mut builder = SettlementBuilder::new(&game.tx(), &update_territory());

        // When
        builder.build(Build::Settlement {
            candidate_positions: candidate_positions.clone(),
            settlement: settlement.clone(),
        });
        let game = game.shutdown();

        // Then
        assert_eq!(
            game.settlements,
            vec![(candidate_positions[1], settlement)]
                .into_iter()
                .collect()
        );
    }

    #[test]
    fn should_not_build_if_all_candidates_controlled() {
        // Given
        let settlement = Settlement {
            position: v2(1, 2),
            ..Settlement::default()
        };
        let control = vec![(settlement.position, v2(0, 0))].into_iter().collect();
        let game = Game {
            control,
            ..Game::default()
        };
        let game = UpdateProcess::new(game);
        let mut builder = SettlementBuilder::new(&game.tx(), &update_territory());

        // When
        builder.build(Build::Settlement {
            candidate_positions: vec![settlement.position],
            settlement,
        });
        let game = game.shutdown();

        // Then
        assert_eq!(game.settlements, HashMap::new());
    }

    #[test]
    fn should_change_settlement_position_if_not_built_on_original_tile() {
        // Given
        let candidate_positions = vec![v2(1, 1), v2(1, 2)];
        let settlement = Settlement {
            position: candidate_positions[0],
            ..Settlement::default()
        };
        let control = vec![(candidate_positions[0], v2(0, 0))]
            .into_iter()
            .collect();
        let game = Game {
            control,
            ..Game::default()
        };
        let game = UpdateProcess::new(game);
        let mut builder = SettlementBuilder::new(&game.tx(), &update_territory());

        // When
        builder.build(Build::Settlement {
            candidate_positions: candidate_positions.clone(),
            settlement: settlement.clone(),
        });
        let game = game.shutdown();

        // Then
        assert_eq!(
            game.settlements,
            vec![(
                candidate_positions[1],
                Settlement {
                    position: candidate_positions[1],
                    ..settlement
                }
            )]
            .into_iter()
            .collect()
        );
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
        let game = UpdateProcess::new(game);
        let update_territory = update_territory();
        let mut builder = SettlementBuilder::new(&game.tx(), &update_territory);

        // When
        builder.build(Build::Settlement {
            candidate_positions: vec![settlement.position],
            settlement,
        });
        game.shutdown();

        // Then
        assert_eq!(*update_territory.lock().unwrap(), vec![v2(1, 2)]);
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
        let game = UpdateProcess::new(game);
        let update_territory = update_territory();
        let mut builder = SettlementBuilder::new(&game.tx(), &update_territory);

        // When
        builder.build(Build::Settlement {
            candidate_positions: vec![settlement.position],
            settlement,
        });
        game.shutdown();

        // Then
        assert_eq!(*update_territory.lock().unwrap(), vec![]);
    }
}
