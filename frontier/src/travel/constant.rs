use super::*;
use std::time::Duration;

pub struct ConstantTravelDuration {
    duration: Duration,
}

impl ConstantTravelDuration {
    pub fn new(duration: Duration) -> ConstantTravelDuration {
        ConstantTravelDuration { duration }
    }

    pub fn boxed(duration: Duration) -> Box<ConstantTravelDuration> {
        Box::new(ConstantTravelDuration::new(duration))
    }
}

impl TravelDuration for ConstantTravelDuration {
    fn get_duration(&self, _: &World, _: &V2<usize>, _: &V2<usize>) -> Option<Duration> {
        Some(self.duration)
    }

    fn min_duration(&self) -> Duration {
        self.duration
    }

    fn max_duration(&self) -> Duration {
        self.duration
    }
}
