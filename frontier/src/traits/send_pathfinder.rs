use commons::async_trait::async_trait;

use crate::pathfinder::Pathfinder;
use crate::travel_duration::TravelDuration;

#[async_trait]
pub trait SendPathfinder {
    type T: TravelDuration + 'static;

    async fn send_pathfinder<F, O>(&self, function: F) -> O
    where
        O: Send + 'static,
        F: FnOnce(&mut Pathfinder<Self::T>) -> O + Send + 'static;

    fn send_pathfinder_background<F, O>(&self, function: F)
    where
        O: Send + 'static,
        F: FnOnce(&mut Pathfinder<Self::T>) -> O + Send + 'static;
}
