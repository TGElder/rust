mod constant;
mod gradient;
mod no_river_corners;

pub use constant::*;
pub use gradient::*;
pub use no_river_corners::*;

use crate::world::World;
use commons::scale::*;
use commons::V2;
use std::iter::{empty, once};
use std::time::Duration;

pub trait TravelDuration: Send + Sync {
    fn get_duration(&self, world: &World, from: &V2<usize>, to: &V2<usize>) -> Option<Duration>;
    fn min_duration(&self) -> Duration;
    fn max_duration(&self) -> Duration;
    fn get_cost_from_duration(&self, duration: &Duration) -> u128 {
        let scale = Scale::<f32>::new(
            (0 as f32, self.max_duration().as_millis() as f32),
            (0.0, 255.0),
        );
        let millis = duration.as_millis() as f32;
        scale.scale(millis).round() as u128
    }

    fn get_cost_from_duration_u8(&self, duration: &Duration) -> u8 {
        let cost = self.get_cost_from_duration(duration);
        if cost > 255 {
            panic!(
                "Duration millis {} must be between 0 and {}",
                duration.as_millis(),
                self.max_duration().as_millis()
            );
        } else {
            cost as u8
        }
    }

    fn get_duration_from_cost(&self, cost: u128) -> Duration {
        let scale = Scale::<f32>::new(
            (0.0, 255.0),
            (0 as f32, self.max_duration().as_millis() as f32),
        );
        Duration::from_millis(scale.scale(cost as f32).round() as u64)
    }

    fn get_cost(&self, world: &World, from: &V2<usize>, to: &V2<usize>) -> Option<u8> {
        self.get_duration(world, from, to)
            .map(|duration| self.get_cost_from_duration_u8(&duration))
    }

    fn get_path_durations<'a>(
        &'a self,
        world: &'a World,
        path: &'a [V2<usize>],
    ) -> Box<dyn Iterator<Item = PathDuration> + 'a> {
        if path.is_empty() {
            return Box::new(empty());
        }
        let iterator = (0..path.len() - 1).flat_map(move |i| {
            let from = path[i];
            let to = path[i + 1];
            once(PathDuration {
                from,
                to,
                duration: self.get_duration(world, &from, &to),
            })
            .chain(once(PathDuration {
                from: to,
                to: from,
                duration: self.get_duration(world, &to, &from),
            }))
        });
        Box::new(iterator)
    }
}

#[derive(Debug, Eq, Hash, PartialEq)]
pub struct PathDuration {
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
    fn test_get_cost_from_duration() {
        let test_duration = TestDuration {
            millis: 1,
            max_millis: 4,
        };
        assert_eq!(
            test_duration.get_cost_from_duration(&Duration::from_millis(12)),
            255 * 3
        );
    }

    #[test]
    fn test_get_cost_from_duration_u8_in_bounds() {
        let test_duration = TestDuration {
            millis: 1,
            max_millis: 4,
        };
        assert_eq!(
            test_duration.get_cost_from_duration_u8(&Duration::from_millis(3)),
            191
        );
    }

    #[test]
    #[should_panic(expected = "Duration millis 5 must be between 0 and 4")]
    fn test_get_cost_from_duration_u8_out_of_bounds() {
        let test_duration = TestDuration {
            millis: 1,
            max_millis: 4,
        };
        test_duration.get_cost_from_duration_u8(&Duration::from_millis(5));
    }

    #[test]
    fn test_get_duration_from_cost() {
        let test_duration = TestDuration {
            millis: 1,
            max_millis: 4,
        };
        assert_eq!(
            test_duration.get_duration_from_cost(255 * 3),
            Duration::from_millis(12)
        );
    }

    #[test]
    fn test_get_duration_from_cost_rounds() {
        let test_duration = TestDuration {
            millis: 1,
            max_millis: 4,
        };
        assert_eq!(
            test_duration.get_duration_from_cost(384),
            Duration::from_millis(6)
        );
    }

    #[test]
    #[rustfmt::skip]
    fn test_get_cost() {
        let travel_duration = TestDuration {
            millis: 1,
            max_millis: 4,
        };
        assert_eq!(
            travel_duration
                .get_cost(&world(), &v2(0, 0), &v2(1, 0))
                .unwrap(),
            64
        );
    }

    #[test]
    #[rustfmt::skip]
    fn test_get_path_durations() {
        
        let travel_duration = TestDuration {
            millis: 1,
            max_millis: 4,
        };
        assert_eq!(
            travel_duration.get_path_durations(&world(), &[v2(0, 0), v2(1, 0), v2(2, 0)]).collect::<HashSet<PathDuration>>(),
            hashset!{ PathDuration{
                from: v2(0, 0),
                to: v2(1, 0),
                duration: Some(Duration::from_millis(1))
            }, PathDuration{
                from: v2(1, 0),
                to: v2(2, 0),
                duration: Some(Duration::from_millis(1))
            }, PathDuration{
                from: v2(2, 0),
                to: v2(1, 0),
                duration: Some(Duration::from_millis(1))
            }, PathDuration{
                from: v2(1, 0),
                to: v2(0, 0),
                duration: Some(Duration::from_millis(1))
            }}
        );
    }

    #[test]
    #[rustfmt::skip]
    fn test_get_path_durations_two_position_path() {

        let travel_duration = TestDuration {
            millis: 1,
            max_millis: 4,
        };
        assert_eq!(
            travel_duration.get_path_durations(&world(), &[v2(0, 0), v2(1, 0)]).collect::<HashSet<PathDuration>>(),
            hashset!{ PathDuration{
                from: v2(0, 0),
                to: v2(1, 0),
                duration: Some(Duration::from_millis(1))
            }, PathDuration{
                from: v2(1, 0),
                to: v2(0, 0),
                duration: Some(Duration::from_millis(1))
            }}
        );
    }

    #[test]
    #[rustfmt::skip]
    fn test_get_path_durations_single_position_path() {

        let travel_duration = TestDuration {
            millis: 1,
            max_millis: 4,
        };
        assert_eq!(
            travel_duration.get_path_durations(&world(), &[v2(0, 0)]).collect::<HashSet<PathDuration>>(),
            hashset!{}
        );
    }

    #[test]
    #[rustfmt::skip]
    fn test_get_path_durations_empty_path() {

        let travel_duration = TestDuration {
            millis: 1,
            max_millis: 4,
        };
        assert_eq!(
            travel_duration.get_path_durations(&world(), &[]).collect::<HashSet<PathDuration>>(),
            hashset!{}
        );
    }
}
