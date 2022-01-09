mod constant;
mod gradient;
mod no_river_corners;

pub use constant::*;
pub use gradient::*;
pub use no_river_corners::*;

use crate::world::World;
use commons::grid::Grid;
use commons::V2;
use serde::{Deserialize, Serialize};
use std::iter::once;
use std::time::Duration;

pub trait TravelDuration: Send + Sync {
    fn get_duration(&self, world: &World, from: &V2<usize>, to: &V2<usize>) -> Option<Duration>;
    fn min_duration(&self) -> Duration;
    fn max_duration(&self) -> Duration;

    fn get_durations_for_position<'a>(
        &'a self,
        world: &'a World,
        position: V2<usize>,
    ) -> Box<dyn Iterator<Item = EdgeDuration> + 'a> {
        let iterator = world
            .neighbours(&position)
            .into_iter()
            .flat_map(move |neighbour| {
                once(EdgeDuration {
                    from: position,
                    to: neighbour,
                    duration: self.get_duration(world, &position, &neighbour),
                })
                .chain(once(EdgeDuration {
                    from: neighbour,
                    to: position,
                    duration: self.get_duration(world, &neighbour, &position),
                }))
            });
        Box::new(iterator)
    }
}

#[derive(Debug, Clone, Eq, Hash, PartialEq, Deserialize, Serialize)]
pub struct EdgeDuration {
    pub from: V2<usize>,
    pub to: V2<usize>,
    pub duration: Option<Duration>,
}

#[cfg(test)]
mod tests {

    use std::collections::HashSet;

    use super::*;
    use commons::{v2, M};

    #[derive(Clone)]
    struct TestDuration {
        millis: u64,
        max_millis: u64,
    }

    impl TravelDuration for TestDuration {
        fn get_duration(&self, _: &World, _: &V2<usize>, _: &V2<usize>) -> Option<Duration> {
            Some(Duration::from_millis(self.millis))
        }

        fn min_duration(&self) -> Duration {
            Duration::from_millis(0)
        }

        fn max_duration(&self) -> Duration {
            Duration::from_millis(self.max_millis)
        }
    }

    fn world() -> World {
        World::new(M::zeros(3, 3), 0.5)
    }

    #[test]
    #[rustfmt::skip]
    fn test_get_durations_for_position() {

        let travel_duration = TestDuration {
            millis: 1,
            max_millis: 4,
        };
        assert_eq!(
            travel_duration.get_durations_for_position(&world(), v2(1, 1)).collect::<HashSet<EdgeDuration>>(),
            hashset!{EdgeDuration{
                from: v2(1, 1),
                to: v2(2, 1),
                duration: Some(Duration::from_millis(1))
            },EdgeDuration{
                from: v2(2, 1),
                to: v2(1, 1),
                duration: Some(Duration::from_millis(1))
            },EdgeDuration{
                from: v2(1, 1),
                to: v2(1, 2),
                duration: Some(Duration::from_millis(1))
            },EdgeDuration{
                from: v2(1, 2),
                to: v2(1, 1),
                duration: Some(Duration::from_millis(1))
            },EdgeDuration{
                from: v2(1, 1),
                to: v2(0, 1),
                duration: Some(Duration::from_millis(1))
            },EdgeDuration{
                from: v2(0, 1),
                to: v2(1, 1),
                duration: Some(Duration::from_millis(1))
            },EdgeDuration{
                from: v2(1, 1),
                to: v2(1, 0),
                duration: Some(Duration::from_millis(1))
            },EdgeDuration{
                from: v2(1, 0),
                to: v2(1, 1),
                duration: Some(Duration::from_millis(1))
            }}
        );
    }

    #[test]
    #[rustfmt::skip]
    fn test_get_durations_for_corner() {

        let travel_duration = TestDuration {
            millis: 1,
            max_millis: 4,
        };
        assert_eq!(
            travel_duration.get_durations_for_position(&world(), v2(0, 0)).collect::<HashSet<EdgeDuration>>(),
            hashset!{EdgeDuration{
                from: v2(0, 0),
                to: v2(1, 0),
                duration: Some(Duration::from_millis(1))
            },EdgeDuration{
                from: v2(1, 0),
                to: v2(0, 0),
                duration: Some(Duration::from_millis(1))
            },EdgeDuration{
                from: v2(0, 0),
                to: v2(0, 1),
                duration: Some(Duration::from_millis(1))
            },EdgeDuration{
                from: v2(0, 1),
                to: v2(0, 0),
                duration: Some(Duration::from_millis(1))
            }}
        );
    }
}
