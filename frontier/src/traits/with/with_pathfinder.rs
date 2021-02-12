use commons::async_trait::async_trait;

use crate::pathfinder::Pathfinder;
use crate::travel_duration::TravelDuration;

#[async_trait]
pub trait WithPathfinder {
    type T: TravelDuration + 'static;

    async fn with_pathfinder<F, O>(&self, function: F) -> O
    where
        F: FnOnce(&Pathfinder<Self::T>) -> O + Send;

    async fn mut_pathfinder<F, O>(&self, function: F) -> O
    where
        F: FnOnce(&mut Pathfinder<Self::T>) -> O + Send;
}
