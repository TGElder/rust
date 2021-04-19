use std::collections::{HashMap, HashSet};

use commons::async_trait::async_trait;
use commons::grid::Grid;
use commons::V2;

use crate::traits::{DrawWorld, WithWorld};
use crate::world::WorldObject;

#[async_trait]
pub trait GetWorldObjects {
    async fn get_world_objects(
        &self,
        positions: &HashSet<V2<usize>>,
    ) -> HashMap<V2<usize>, WorldObject>;
}

#[async_trait]
pub trait SetWorldObjects {
    async fn set_world_objects(&self, objects: &HashMap<V2<usize>, WorldObject>);
}

#[async_trait]
pub trait RemoveWorldObjects {
    async fn remove_world_objects(&self, positions: &HashSet<V2<usize>>);
}

#[async_trait]
impl<T> GetWorldObjects for T
where
    T: WithWorld + Sync,
{
    async fn get_world_objects(
        &self,
        positions: &HashSet<V2<usize>>,
    ) -> HashMap<V2<usize>, WorldObject> {
        self.with_world(|world| {
            positions
                .iter()
                .flat_map(|position| {
                    world
                        .get_cell(position)
                        .map(|cell| (cell.position, cell.object))
                })
                .collect()
        })
        .await
    }
}

#[async_trait]
impl<T> SetWorldObjects for T
where
    T: DrawWorld + WithWorld + Sync,
{
    async fn set_world_objects(&self, objects: &HashMap<V2<usize>, WorldObject>) {
        self.mut_world(|world| {
            for (position, object) in objects {
                let cell = unwrap_or!(world.mut_cell(position), continue);
                cell.object = *object
            }
        })
        .await;

        let tiles = objects.keys().copied().collect();
        self.draw_world_tiles(tiles).await;
    }
}

#[async_trait]
impl<T> RemoveWorldObjects for T
where
    T: DrawWorld + WithWorld + Sync,
{
    async fn remove_world_objects(&self, positions: &HashSet<V2<usize>>) {
        self.set_world_objects(
            &positions
                .iter()
                .map(|position| (*position, WorldObject::None))
                .collect(),
        )
        .await;
    }
}
