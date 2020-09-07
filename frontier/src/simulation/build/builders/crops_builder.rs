use super::*;

use crate::game::traits::{BuildCrops, Settlements};
use crate::settlement::{Settlement, SettlementClass::Town};
use commons::V2;

const HANDLE: &str = "crops_builder";

pub struct CropsBuilder<G>
where
    G: BuildCrops + Settlements,
{
    game: UpdateSender<G>,
}

impl<G> Builder for CropsBuilder<G>
where
    G: BuildCrops + Settlements,
{
    fn can_build(&self, build: &Build) -> bool {
        if let Build::Crops { .. } = build {
            true
        } else {
            false
        }
    }

    fn build(&mut self, build: Build) {
        if let Build::Crops { position, rotated } = build {
            self.try_build_crops(position, rotated);
        }
    }
}

impl<G> CropsBuilder<G>
where
    G: BuildCrops + Settlements,
{
    pub fn new(game: &UpdateSender<G>) -> CropsBuilder<G> {
        CropsBuilder {
            game: game.clone_with_handle(HANDLE),
        }
    }

    fn try_build_crops(&mut self, position: V2<usize>, rotated: bool) {
        block_on(async {
            self.game
                .update(move |game| try_build_crops(game, position, rotated))
                .await
        })
    }
}

fn try_build_crops<G>(game: &mut G, position: V2<usize>, rotated: bool)
where
    G: BuildCrops + Settlements,
{
    if let Some(Settlement { class: Town, .. }) = game.get_settlement(&position) {
        return;
    }
    game.build_crops(&position, rotated);
}

#[cfg(test)]
mod tests {
    use super::*;

    use commons::update::UpdateProcess;
    use commons::v2;
    use std::collections::HashMap;

    struct MockGame {
        crops: HashMap<V2<usize>, bool>,
        settlements: HashMap<V2<usize>, Settlement>,
    }

    impl Default for MockGame {
        fn default() -> MockGame {
            MockGame {
                crops: hashmap! {},
                settlements: hashmap! {},
            }
        }
    }

    impl BuildCrops for MockGame {
        fn build_crops(&mut self, position: &V2<usize>, rotated: bool) -> bool {
            self.crops.insert(*position, rotated);
            true
        }
    }

    impl Settlements for MockGame {
        fn settlements(&self) -> &HashMap<V2<usize>, Settlement> {
            &self.settlements
        }
    }

    #[test]
    fn can_build_crops() {
        // Given
        let game = UpdateProcess::new(MockGame::default());
        let builder = CropsBuilder::new(&game.tx());

        // When
        let can_build = builder.can_build(&Build::Crops {
            position: v2(1, 2),
            rotated: true,
        });

        // Then
        assert!(can_build);

        // Finally
        game.shutdown();
    }

    #[test]
    fn should_build_crops() {
        // Given
        let game = UpdateProcess::new(MockGame::default());
        let mut builder = CropsBuilder::new(&game.tx());

        // When
        builder.build(Build::Crops {
            position: v2(1, 2),
            rotated: true,
        });

        // Then
        let game = game.shutdown();
        assert_eq!(game.crops, hashmap! {v2(1, 2) => true});
    }

    #[test]
    fn should_not_build_crops_if_town_on_tile() {
        // Given
        let settlement = Settlement {
            position: v2(1, 2),
            class: Town,
            ..Settlement::default()
        };
        let game = MockGame {
            settlements: hashmap! {v2(1, 2) => settlement},
            ..MockGame::default()
        };
        let game = UpdateProcess::new(game);
        let mut builder = CropsBuilder::new(&game.tx());

        // When
        builder.build(Build::Crops {
            position: v2(1, 2),
            rotated: true,
        });

        // Then
        let game = game.shutdown();
        assert_eq!(game.crops, hashmap! {});
    }
}
