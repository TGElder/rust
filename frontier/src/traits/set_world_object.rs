use commons::async_trait::async_trait;
use commons::{Grid, V2};

use crate::traits::{Micros, Redraw, SendWorld};
use crate::world::{World, WorldObject};

#[async_trait]
pub trait SetWorldObject {
    async fn set_world_object(&self, object: WorldObject, position: V2<usize>) -> bool;
    async fn force_world_object(&self, object: WorldObject, position: V2<usize>);
}

#[async_trait]
impl<T> SetWorldObject for T
where
    T: Micros + Redraw + SendWorld + Sync,
{
    async fn set_world_object(&self, object: WorldObject, position: V2<usize>) -> bool {
        if send_set_world_object(self, object, position, false).await {
            let when = self.micros().await;
            self.redraw_tile_at(position, when);
            true
        } else {
            false
        }
    }

    async fn force_world_object(&self, object: WorldObject, position: V2<usize>) {
        send_set_world_object(self, object, position, true).await;
        let when = self.micros().await;
        self.redraw_tile_at(position, when);
    }
}

async fn send_set_world_object<T>(
    x: &T,
    object: WorldObject,
    position: V2<usize>,
    force: bool,
) -> bool
where
    T: SendWorld,
{
    x.send_world(move |world| set_world_object(world, object, position, force))
        .await
}

fn set_world_object(
    world: &mut World,
    object: WorldObject,
    position: V2<usize>,
    force: bool,
) -> bool {
    let cell = unwrap_or!(world.mut_cell(&position), return false);
    if !force && cell.object != WorldObject::None {
        return false;
    }
    cell.object = object;
    true
}
