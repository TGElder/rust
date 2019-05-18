mod constant;
mod gradient;

pub use self::constant::*;
pub use self::gradient::*;

use crate::world::World;
use commons::scale::*;
use commons::V2;
use std::time::Duration;

pub trait TravelDuration {
    fn get_duration(&self, world: &World, from: &V2<usize>, to: &V2<usize>) -> Option<Duration>;
    fn max_duration(&self) -> Duration;
}

impl TravelDuration {
    pub fn get_cost(&self, world: &World, from: &V2<usize>, to: &V2<usize>) -> Option<u8> {
        self.get_duration(world, from, to).map(|duration| {
            let scale = Scale::<f32>::new(
                (0 as f32, self.max_duration().as_millis() as f32),
                (0.0, 255.0),
            );
            let millis = duration.as_millis() as f32;
            if !scale.inside_range(millis) {
                panic!(
                    "Duration millis {} not in expected range {:?}",
                    millis,
                    scale.in_range()
                );
            }
            scale.scale(millis).round() as u8
        })
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use commons::{v2, M};
    use std::time::Instant;

    struct TestDuration {
        millis: u64,
        max_millis: u64,
    }

    impl TravelDuration for TestDuration {
        fn get_duration(&self, _: &World, _: &V2<usize>, _: &V2<usize>) -> Option<Duration> {
            Some(Duration::from_millis(self.millis))
        }

        fn max_duration(&self) -> Duration {
            Duration::from_millis(self.max_millis)
        }
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
            vec![],
            vec![],
            0.5,
            Instant::now(),
        );
        let travel_duration: Box<TravelDuration> = Box::new(TestDuration {
            millis: 1,
            max_millis: 4,
        });
        assert_eq!(
            travel_duration
                .get_cost(&world, &v2(0, 0), &v2(1, 0))
                .unwrap(),
            64
        );
    }

}
