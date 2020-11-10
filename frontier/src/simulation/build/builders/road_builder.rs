use super::*;

use crate::game::traits::BuildRoad;
use commons::edge::Edge;

const HANDLE: &str = "road_builder";

pub struct RoadBuilder<G>
where
    G: BuildRoad,
{
    game: FnSender<G>,
}

#[async_trait]
impl<G> Builder for RoadBuilder<G>
where
    G: BuildRoad,
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
    G: BuildRoad,
{
    pub fn new(game: &FnSender<G>) -> RoadBuilder<G> {
        RoadBuilder {
            game: game.clone_with_name(HANDLE),
        }
    }

    async fn build_road(&mut self, road: Edge) {
        self.game.send(move |game| build_road(game, road)).await
    }
}

fn build_road<G>(game: &mut G, road: Edge)
where
    G: BuildRoad,
{
    game.add_road(&road);
}

#[cfg(test)]
mod tests {
    use super::*;

    use commons::fn_sender::FnThread;
    use commons::futures::executor::block_on;
    use commons::v2;

    #[test]
    fn can_build_road() {
        // Given
        let game = FnThread::new(hashset! {});
        let builder = RoadBuilder::new(&game.tx());

        // When
        let can_build = builder.can_build(&Build::Road(Edge::new(v2(1, 2), v2(1, 3))));

        // Then
        assert!(can_build);

        // Finally
        game.join();
    }

    #[test]
    fn should_build_road() {
        // Given
        let game = FnThread::new(hashset! {});
        let mut builder = RoadBuilder::new(&game.tx());

        // When
        block_on(builder.build(Build::Road(Edge::new(v2(1, 2), v2(1, 3)))));

        // Then
        let roads = game.join();
        assert_eq!(roads, hashset! {Edge::new(v2(1, 2), v2(1, 3))});
    }
}
