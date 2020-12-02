use commons::async_trait::async_trait;
use commons::V2;

use crate::traits::SetWorldObject;
use crate::world::WorldObject;

#[async_trait]
pub trait AddCrops {
    async fn add_crops(&self, position: V2<usize>, rotated: bool) -> bool;
}

#[async_trait]
impl<T> AddCrops for T
where
    T: SetWorldObject + Send + Sync,
{
    async fn add_crops(&self, position: V2<usize>, rotated: bool) -> bool {
        self.set_world_object(WorldObject::Crop { rotated }, position)
            .await
    }
}
