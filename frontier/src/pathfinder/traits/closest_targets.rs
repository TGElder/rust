use commons::V2;
use std::time::Duration;

#[derive(Debug, PartialEq)]
pub struct ClosestTargetResult {
    pub position: V2<usize>,
    pub path: Vec<V2<usize>>,
    pub duration: Duration,
}

pub trait ClosestTargets {
    fn init_targets(&mut self, name: String);

    fn load_target(&mut self, name: &str, position: &V2<usize>, target: bool);

    fn closest_targets(
        &self,
        positions: &[V2<usize>],
        targets: &str,
        n_closest: usize,
    ) -> Vec<ClosestTargetResult>;
}
