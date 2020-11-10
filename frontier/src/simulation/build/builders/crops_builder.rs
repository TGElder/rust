use super::*;

use crate::game::traits::{BuildCrops, Settlements};
use crate::settlement::{Settlement, SettlementClass::Town};
use commons::V2;

const HANDLE: &str = "crops_builder";

pub struct CropsBuilder<G>
where
    G: BuildCrops + Settlements,
{
    game: FnSender<G>,
}

#[async_trait]
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

    async fn build(&mut self, build: Build) {
        if let Build::Crops { position, rotated } = build {
            self.try_build_crops(position, rotated).await;
        }
    }
}

impl<G> CropsBuilder<G>
where
    G: BuildCrops + Settlements,
{
    pub fn new(game: &FnSender<G>) -> CropsBuilder<G> {
        CropsBuilder {
            game: game.clone_with_name(HANDLE),
        }
    }

    async fn try_build_crops(&mut self, position: V2<usize>, rotated: bool) {
        self.game
            .send(move |game| try_build_crops(game, position, rotated))
            .await
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

    use commons::fn_sender::FnThread;
    use commons::futures::executor::block_on;
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
        let game = FnThread::new(MockGame::default());
        let builder = CropsBuilder::new(&game.tx());

        // When
        let can_build = builder.can_build(&Build::Crops {
            position: v2(1, 2),
            rotated: true,
        });

        // Then
        assert!(can_build);

        // Finally
        game.join();
    }

    #[test]
    fn should_build_crops_if_no_town_on_tile() {
        // Given
        let game = FnThread::new(MockGame::default());
        let mut builder = CropsBuilder::new(&game.tx());

        // When
        block_on(builder.build(Build::Crops {
            position: v2(1, 2),
            rotated: true,
        }));

        // Then
        let game = game.join();
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
        let game = FnThread::new(game);
        let mut builder = CropsBuilder::new(&game.tx());

        // When
        block_on(builder.build(Build::Crops {
            position: v2(1, 2),
            rotated: true,
        }));

        // Then
        let game = game.join();
        assert_eq!(game.crops, hashmap! {});
    }
}
