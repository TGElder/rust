mod constant;
mod gradient;
mod no_river_corners;

pub use constant::*;
pub use gradient::*;
pub use no_river_corners::*;

use crate::world::World;
use commons::grid::Grid;
use commons::V2;
use commons::{scale::*, v2};
use serde::{Deserialize, Serialize};
use std::iter::once;
use std::time::Duration;
use std::usize;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Deserialize, Serialize)]
pub struct TravelPosition {
    pub x: u16,
    pub y: u16,
    pub mode: TravelMode,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Deserialize, Serialize)]
pub enum TravelMode {
    Land = 0,
    Water = 1,
}

pub fn land(x: u16, y: u16) -> TravelPosition {
    TravelPosition {
        x,
        y,
        mode: TravelMode::Land,
    }
}

pub fn water(x: u16, y: u16) -> TravelPosition {
    TravelPosition {
        x,
        y,
        mode: TravelMode::Water,
    }
}

impl From<&TravelPosition> for V2<usize> {
    fn from(position: &TravelPosition) -> Self {
        v2(position.x as usize, position.y as usize)
    }
}

impl From<TravelPosition> for V2<usize> {
    fn from(position: TravelPosition) -> Self {
        v2(position.x as usize, position.y as usize)
    }
}

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

    fn get_durations_for_position<'a>(
        &'a self,
        world: &'a World,
        position: V2<usize>,
    ) -> Box<dyn Iterator<Item = EdgeDuration> + 'a> {
        let from = land(position.x as u16, position.y as u16);
        let iterator = world
            .neighbours(&position)
            .into_iter()
            .flat_map(move |neighbour| {
                let to = land(neighbour.x as u16, neighbour.y as u16);
                once(EdgeDuration {
                    from,
                    to,
                    duration: self.get_duration(world, &position, &neighbour),
                })
                .chain(once(EdgeDuration {
                    from: to,
                    to: from,
                    duration: self.get_duration(world, &neighbour, &position),
                }))
            });
        Box::new(iterator)
    }
}

#[derive(Debug, Eq, Hash, PartialEq)]
pub struct EdgeDuration {
    pub from: TravelPosition,
    pub to: TravelPosition,
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
    fn test_get_durations_for_position() {

        let travel_duration = TestDuration {
            millis: 1,
            max_millis: 4,
        };
        assert_eq!(
            travel_duration.get_durations_for_position(&world(), v2(1, 1)).collect::<HashSet<EdgeDuration>>(),
            hashset!{EdgeDuration{
                from: land(1, 1),
                to: land(2, 1),
                duration: Some(Duration::from_millis(1))
            },EdgeDuration{
                from: land(2, 1),
                to: land(1, 1),
                duration: Some(Duration::from_millis(1))
            },EdgeDuration{
                from: land(1, 1),
                to: land(1, 2),
                duration: Some(Duration::from_millis(1))
            },EdgeDuration{
                from: land(1, 2),
                to: land(1, 1),
                duration: Some(Duration::from_millis(1))
            },EdgeDuration{
                from: land(1, 1),
                to: land(0, 1),
                duration: Some(Duration::from_millis(1))
            },EdgeDuration{
                from: land(0, 1),
                to: land(1, 1),
                duration: Some(Duration::from_millis(1))
            },EdgeDuration{
                from: land(1, 1),
                to: land(1, 0),
                duration: Some(Duration::from_millis(1))
            },EdgeDuration{
                from: land(1, 0),
                to: land(1, 1),
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
                from: land(0, 0),
                to: land(1, 0),
                duration: Some(Duration::from_millis(1))
            },EdgeDuration{
                from: land(1, 0),
                to: land(0, 0),
                duration: Some(Duration::from_millis(1))
            },EdgeDuration{
                from: land(0, 0),
                to: land(0, 1),
                duration: Some(Duration::from_millis(1))
            },EdgeDuration{
                from: land(0, 1),
                to: land(0, 0),
                duration: Some(Duration::from_millis(1))
            }}
        );
    }
}
