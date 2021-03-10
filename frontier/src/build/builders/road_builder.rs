use crate::traits::AddRoad;

use super::*;

use commons::edge::Edge;
use commons::log::debug;

pub struct RoadBuilder<T>
where
    T: AddRoad + Send + Sync,
{
    cx: T,
}

#[async_trait]
impl<T> Builder for RoadBuilder<T>
where
    T: AddRoad + Send + Sync,
{
    fn can_build(&self, build: &Build) -> bool {
        matches!(build, Build::Road(..))
    }

    async fn build(&mut self, build: Vec<Build>) {
        let roads = build.into_iter().flat_map(|build| {
            match build {
                Build::Road(road) => Some(road),
                _ => None
            }
        }).collect::<Vec<_>>();
        self.cx.add_roads(&roads).await;
    }
}

impl<T> RoadBuilder<T>
where
    T: AddRoad + Send + Sync,
{
    pub fn new(game: T) -> RoadBuilder<T> {
        RoadBuilder { cx: game }
    }

    async fn build_road(&mut self, road: &Edge) {
        // debug!("Building road");
        self.cx.add_roads(&[*road]).await;
        // debug!("Built road");
    }
}

// #[cfg(test)]
// mod tests {
//     use std::collections::HashSet;
//     use std::sync::{Arc, Mutex};

//     use super::*;

//     use commons::{v2, Arm};
//     use futures::executor::block_on;

//     #[async_trait]
//     impl AddRoad for Arm<HashSet<Edge>> {
//         async fn add_roads(&self, edges: &[Edge]) {
//             for edge in edges {
//                 self.lock().unwrap().insert(*edge);
//             }
//         }
//     }

//     #[test]
//     fn can_build_road() {
//         // Given
//         let game = Arc::new(Mutex::new(hashset! {}));
//         let builder = RoadBuilder::new(game);

//         // When
//         let can_build = builder.can_build(&Build::Road(Edge::new(v2(1, 2), v2(1, 3))));

//         // Then
//         assert!(can_build);
//     }

//     #[test]
//     fn should_build_road() {
//         // Given
//         let game = Arc::new(Mutex::new(hashset! {}));
//         let mut builder = RoadBuilder::new(game);

//         // When
//         block_on(builder.build(Build::Road(Edge::new(v2(1, 2), v2(1, 3)))));

//         // Then
//         assert_eq!(
//             *builder.cx.lock().unwrap(),
//             hashset! {Edge::new(v2(1, 2), v2(1, 3))}
//         );
//     }
// }
