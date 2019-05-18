use super::*;
use commons::scale::*;

pub struct GradientTravelDuration {
    rise_to_millis: Scale<f32>,
    use_absolute_rise: bool,
}

impl GradientTravelDuration {
    pub fn new(rise_to_millis: Scale<f32>, use_absolute_rise: bool) -> GradientTravelDuration {
        GradientTravelDuration {
            rise_to_millis,
            use_absolute_rise,
        }
    }

    pub fn boxed(
        rise_to_millis: Scale<f32>,
        use_absolute_rise: bool,
    ) -> Box<GradientTravelDuration> {
        Box::new(GradientTravelDuration::new(
            rise_to_millis,
            use_absolute_rise,
        ))
    }
}

impl TravelDuration for GradientTravelDuration {
    fn get_duration(&self, world: &World, from: &V2<usize>, to: &V2<usize>) -> Option<Duration> {
        world.get_rise(from, to).and_then(|rise| {
            let rise = if self.use_absolute_rise {
                rise.abs()
            } else {
                rise
            };
            if self.rise_to_millis.inside_range(rise) {
                let millis = self.rise_to_millis.scale(rise) as u64;
                Some(Duration::from_millis(millis))
            } else {
                None
            }
        })
    }

    fn max_duration(&self) -> Duration {
        Duration::from_millis(self.rise_to_millis.out_range().1 as u64)
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::world::World;
    use commons::{v2, M};
    use std::time::Instant;

    fn world() -> World {
        World::new(
            M::from_vec(3, 3, vec![1.0, 1.0, 3.0, 1.0, 1.0, 3.0, 1.0, 1.5, 3.0]),
            vec![],
            vec![],
            2.0,
            Instant::now(),
        )
    }

    fn scale() -> Scale<f32> {
        Scale::new((-1.0, 1.0), (10.0, 110.0))
    }

    fn gradient_travel_duration(use_absolute_rise: bool) -> GradientTravelDuration {
        GradientTravelDuration::new(scale(), use_absolute_rise)
    }

    #[test]
    fn uphill_do_not_use_absolute_rise() {
        assert_eq!(
            gradient_travel_duration(false).get_duration(&world(), &v2(1, 1), &v2(1, 2)),
            Some(Duration::from_millis(85))
        );
    }

    #[test]
    fn downhill_do_not_use_absolute_rise() {
        assert_eq!(
            gradient_travel_duration(false).get_duration(&world(), &v2(1, 2), &v2(1, 1)),
            Some(Duration::from_millis(35))
        );
    }

    #[test]
    fn downhill_use_absolute_rise() {
        assert_eq!(
            gradient_travel_duration(true).get_duration(&world(), &v2(1, 2), &v2(1, 1)),
            Some(Duration::from_millis(85))
        );
    }

    #[test]
    fn max_duration() {
        assert_eq!(
            gradient_travel_duration(true).max_duration(),
            Duration::from_millis(110)
        );
    }

    #[test]
    fn should_not_allow_gradient_out_of_range_uphill() {
        assert_eq!(
            gradient_travel_duration(true).get_duration(&world(), &v2(1, 0), &v2(2, 0)),
            None
        );
    }

    #[test]
    fn should_not_allow_gradient_out_of_range_downhill() {
        assert_eq!(
            gradient_travel_duration(true).get_duration(&world(), &v2(2, 0), &v2(1, 0)),
            None
        );
    }
}
