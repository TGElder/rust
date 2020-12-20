use std::collections::HashSet;

use commons::grid::Grid;
use commons::log::trace;

use crate::resource::Resource;
use crate::traits::{
    GetSettlement, RandomTownName, RemoveWorldObject, SendGame, SendWorld, WhoControlsTile,
};
use crate::world::{World, WorldObject};

use super::*;
pub struct TryRemoveCrops<X> {
    x: X,
}

#[async_trait]
impl<X> Processor for TryRemoveCrops<X>
where
    X: RemoveWorldObject + SendWorld + Send + Sync + 'static,
{
    async fn process(&mut self, mut state: State, instruction: &Instruction) -> State {
        let mut positions = match instruction {
            Instruction::RefreshPositions(positions) => positions.clone(),
            _ => return state,
        };

        let start = std::time::Instant::now();
        let position_count = positions.len();

        positions.retain(|position| has_no_crop_routes(&state, position));
        let have_crops = self
            .x
            .send_world(move |world| have_crops(world, positions))
            .await;

        let removed = have_crops.len();
        for position in have_crops {
            state.build_queue.remove(&BuildKey::Crops(position));
            self.x.remove_world_object(position).await; // TODO trait that checks the world object before removing
        }

        trace!(
            "Removed {}/{} crops in {}ms",
            removed,
            position_count,
            start.elapsed().as_millis()
        );

        state
    }
}

impl<X> TryRemoveCrops<X>
where
    X: GetSettlement + RandomTownName + SendGame + SendWorld + WhoControlsTile,
{
    pub fn new(x: X) -> TryRemoveCrops<X> {
        TryRemoveCrops { x }
    }
}

fn has_no_crop_routes(state: &State, position: &V2<usize>) -> bool {
    !ok_or!(state.traffic.get(&position), return false)
        .iter()
        .any(|route| route.resource == Resource::Crops && route.destination == *position)
}

fn have_crops(world: &World, positions: HashSet<V2<usize>>) -> Vec<V2<usize>> {
    positions
        .into_iter()
        .filter(|position| has_crops(world, position))
        .collect()
}

fn has_crops(world: &World, position: &V2<usize>) -> bool {
    world.get_cell(&position).map_or(false, |cell| {
        if let WorldObject::Crop { .. } = cell.object {
            true
        } else {
            false
        }
    })
}
