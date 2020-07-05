use commons::V2;
use std::collections::HashMap;
use std::time::Duration;

pub trait PositionsWithin {
    fn positions_within(
        &self,
        positions: &[V2<usize>],
        duration: Duration,
    ) -> HashMap<V2<usize>, Duration>;
}
