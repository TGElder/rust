use super::*;

use crate::game::traits::BuildRoad;
use commons::edge::Edge;

const HANDLE: &str = "road_builder";

pub struct RoadBuilder<G>
where
    G: BuildRoad,
{
    game: UpdateSender<G>,
}

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

    fn build(&mut self, build: Build) {
        if let Build::Road(road) = build {
            self.build_road(road);
        }
    }
}

impl<G> RoadBuilder<G>
where
    G: BuildRoad,
{
    pub fn new(game: &UpdateSender<G>) -> RoadBuilder<G> {
        RoadBuilder {
            game: game.clone_with_handle(HANDLE),
        }
    }

    fn build_road(&mut self, road: Edge) {
        sync!(self.game.update(move |game| build_road(game, road)))
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

    use commons::update::UpdateProcess;
    use commons::v2;

    #[test]
    fn can_build_road() {
        // Given
        let game = UpdateProcess::new(hashset! {});
        let builder = RoadBuilder::new(&game.tx());

        // When
        let can_build = builder.can_build(&Build::Road(Edge::new(v2(1, 2), v2(1, 3))));

        // Then
        assert!(can_build);

        // Finally
        game.shutdown();
    }

    #[test]
    fn should_build_road() {
        // Given
        let game = UpdateProcess::new(hashset! {});
        let mut builder = RoadBuilder::new(&game.tx());

        // When
        builder.build(Build::Road(Edge::new(v2(1, 2), v2(1, 3))));

        // Then
        let roads = game.shutdown();
        assert_eq!(roads, hashset! {Edge::new(v2(1, 2), v2(1, 3))});
    }
}
