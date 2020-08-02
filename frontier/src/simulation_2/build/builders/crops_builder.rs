use super::*;

use crate::game::traits::BuildCrops;
use commons::V2;

const HANDLE: &str = "crops_builder";

pub struct CropsBuilder<G>
where
    G: BuildCrops,
{
    game: UpdateSender<G>,
}

impl<G> Builder for CropsBuilder<G>
where
    G: BuildCrops,
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
            self.build_crops(position, rotated);
        }
    }
}

impl<G> CropsBuilder<G>
where
    G: BuildCrops,
{
    pub fn new(game: &UpdateSender<G>) -> CropsBuilder<G> {
        CropsBuilder {
            game: game.clone_with_handle(HANDLE),
        }
    }

    fn build_crops(&mut self, position: V2<usize>, rotated: bool) {
        block_on(async {
            self.game
                .update(move |game| build_crops(game, position, rotated))
                .await
        })
    }
}

fn build_crops<G>(game: &mut G, position: V2<usize>, rotated: bool)
where
    G: BuildCrops,
{
    game.build_crops(&position, rotated);
}

#[cfg(test)]
mod tests {
    use super::*;

    use commons::update::UpdateProcess;
    use commons::v2;

    #[test]
    fn can_build_crops() {
        // Given
        let game = UpdateProcess::new(hashmap! {});
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
        let game = UpdateProcess::new(hashmap! {});
        let mut builder = CropsBuilder::new(&game.tx());

        // When
        builder.build(Build::Crops {
            position: v2(1, 2),
            rotated: true,
        });

        // Then
        let crops = game.shutdown();
        assert_eq!(crops, hashmap! {v2(1, 2) => true});
    }
}
