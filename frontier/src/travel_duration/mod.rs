mod constant;
mod gradient;

pub use self::constant::*;
pub use self::gradient::*;

use crate::world::World;
use commons::scale::*;
use commons::V2;
use std::time::Duration;

pub trait TravelDuration: Send + Sync {
    fn get_duration(&self, world: &World, from: &V2<usize>, to: &V2<usize>) -> Option<Duration>;
    fn min_duration(&self) -> Duration;
    fn max_duration(&self) -> Duration;
    fn get_cost_from_duration(&self, duration: Duration) -> u128 {
        let scale = Scale::<f32>::new(
            (0 as f32, self.max_duration().as_millis() as f32),
            (0.0, 255.0),
        );
        let millis = duration.as_millis() as f32;
        scale.scale(millis).round() as u128
    }

    fn get_cost_from_duration_u8(&self, duration: Duration) -> u8 {
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
}

pub trait TravelCost {
    fn get_cost(&self, world: &World, from: &V2<usize>, to: &V2<usize>) -> Option<u8>;
}

impl<T> TravelCost for T
where
    T: TravelDuration,
{
    fn get_cost(&self, world: &World, from: &V2<usize>, to: &V2<usize>) -> Option<u8> {
        self.get_duration(world, from, to)
            .map(|duration| self.get_cost_from_duration_u8(duration))
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use commons::{v2, M};

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

    #[test]
    fn test_get_cost_from_duration() {
        let test_duration = TestDuration {
            millis: 1,
            max_millis: 4,
        };
        assert_eq!(
            test_duration.get_cost_from_duration(Duration::from_millis(12)),
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
            test_duration.get_cost_from_duration_u8(Duration::from_millis(3)),
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
        test_duration.get_cost_from_duration_u8(Duration::from_millis(5));
    }

    #[test]
    #[rustfmt::skip]
    fn test_get_cost() {
        let world = World::new(
            M::from_vec(3, 3, 
            vec![
                1.0, 1.0, 1.0,
                1.0, 1.0, 1.0,
                1.0, 1.0, 1.0
            ]),
            0.5,
        );
        let travel_duration = TestDuration {
            millis: 1,
            max_millis: 4,
        };
        assert_eq!(
            travel_duration
                .get_cost(&world, &v2(0, 0), &v2(1, 0))
                .unwrap(),
            64
        );
    }
}
