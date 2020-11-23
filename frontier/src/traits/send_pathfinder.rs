use commons::async_trait::async_trait;

use crate::pathfinder::Pathfinder;
use crate::travel_duration::TravelDuration;

#[async_trait]
pub trait SendPathfinder<T> 
    where T: TravelDuration
{
    async fn send_pathfinder<F, O>(&mut self, function: F) -> O
    where
        O: Send + 'static,
        F: FnOnce(&mut Pathfinder<T>) -> O + Send + 'static;

    fn send_pathfinder_background<F, O>(&mut self, function: F)
    where
        O: Send + 'static,
        F: FnOnce(&mut Pathfinder<T>) -> O + Send + 'static;

}