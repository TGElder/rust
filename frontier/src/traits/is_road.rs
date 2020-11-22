use commons::async_trait::async_trait;

use commons::edge::Edge;

use crate::traits::SendWorld;

#[async_trait]
pub trait IsRoad {
    async fn is_road(&mut self, edge: Edge) -> bool;
}

#[async_trait]
impl<T> IsRoad for T
where
    T: SendWorld + Send,
{
    async fn is_road(&mut self, edge: Edge) -> bool {
        self.send_world(move |world| world.is_road(&edge)).await
    }
}
