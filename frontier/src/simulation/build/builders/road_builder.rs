use super::*;

use crate::game::traits::BuildRoad;
use commons::edge::Edge;

pub struct RoadBuilder<G>
where
    G: BuildRoad + Send,
{
    game: G,
}

#[async_trait]
impl<G> Builder for RoadBuilder<G>
where
    G: BuildRoad + Send,
{
    fn can_build(&self, build: &Build) -> bool {
        if let Build::Road(..) = build {
            true
        } else {
            false
        }
    }

    async fn build(&mut self, build: Build) {
        if let Build::Road(road) = build {
            self.build_road(road).await;
        }
    }
}

impl<G> RoadBuilder<G>
where
    G: BuildRoad + Send,
{
    pub fn new(game: G) -> RoadBuilder<G> {
        RoadBuilder { game }
    }

    async fn build_road(&mut self, road: Edge) {
        self.game.add_road(&road).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use commons::futures::executor::block_on;
    use commons::v2;

    #[test]
    fn can_build_road() {
        // Given
        let game = hashset! {};
        let builder = RoadBuilder::new(game);

        // When
        let can_build = builder.can_build(&Build::Road(Edge::new(v2(1, 2), v2(1, 3))));

        // Then
        assert!(can_build);
    }

    #[test]
    fn should_build_road() {
        // Given
        let game = hashset! {};
        let mut builder = RoadBuilder::new(game);

        // When
        block_on(builder.build(Build::Road(Edge::new(v2(1, 2), v2(1, 3)))));

        // Then
        assert_eq!(builder.game, hashset! {Edge::new(v2(1, 2), v2(1, 3))});
    }
}
