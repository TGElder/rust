use commons::V2;
use std::time::Duration;

pub trait LowestDuration {
    fn lowest_duration(&self, path: &[V2<usize>]) -> Option<Duration>;
}
