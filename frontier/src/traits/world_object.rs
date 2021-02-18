use commons::async_trait::async_trait;
use commons::grid::Grid;
use commons::V2;

use crate::traits::{DrawWorld, Micros, WithWorld};
use crate::world::{World, WorldObject};

#[async_trait]
pub trait SetWorldObject {
    async fn set_world_object(&self, object: WorldObject, position: &V2<usize>) -> bool;
}

#[async_trait]
pub trait ForceWorldObject {
    async fn force_world_object(&self, object: WorldObject, position: &V2<usize>);
}

#[async_trait]
pub trait RemoveWorldObject {
    async fn remove_world_object(&self, position: &V2<usize>);
}

#[async_trait]
impl<T> SetWorldObject for T
where
    T: DrawWorld + Micros + WithWorld + Sync,
{
    async fn set_world_object(&self, object: WorldObject, position: &V2<usize>) -> bool {
        if send_set_world_object(self, object, position, true).await {
            let when = self.micros().await;
            self.draw_world_tile(*position, when);
            true
        } else {
            false
        }
    }
}

#[async_trait]
impl<T> ForceWorldObject for T
where
    T: DrawWorld + Micros + WithWorld + Sync,
{
    async fn force_world_object(&self, object: WorldObject, position: &V2<usize>) {
        send_set_world_object(self, object, position, false).await;
        let when = self.micros().await;
        self.draw_world_tile(*position, when);
    }
}

#[async_trait]
impl<T> RemoveWorldObject for T
where
    T: DrawWorld + Micros + WithWorld + Sync,
{
    async fn remove_world_object(&self, position: &V2<usize>) {
        self.force_world_object(WorldObject::None, position).await;
    }
}

async fn send_set_world_object<T>(
    cx: &T,
    object: WorldObject,
    position: &V2<usize>,
    force: bool,
) -> bool
where
    T: WithWorld,
{
    cx.mut_world(|world| set_world_object(world, object, position, force))
        .await
}

fn set_world_object(
    world: &mut World,
    object: WorldObject,
    position: &V2<usize>,
    check_is_empty: bool,
) -> bool {
    let cell = unwrap_or!(world.mut_cell(position), return false);
    if check_is_empty && cell.object != WorldObject::None {
        return false;
    }
    cell.object = object;
    true
}
