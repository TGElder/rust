use std::collections::HashSet;

use commons::grid::Grid;
use commons::log::info;

use crate::game::traits::GetRoute;
use crate::resource::Resource;
use crate::route::Route;
use crate::traits::{SendGame, SendWorld};
use crate::world::{World, WorldObject};

use super::*;
pub struct TryBuildCrops<X> {
    x: X,
}

#[async_trait]
impl<X> Processor for TryBuildCrops<X>
where
    X: SendWorld + SendGame + Send + Sync + 'static,
{
    async fn process(&mut self, mut state: State, instruction: &Instruction) -> State {
        let mut positions = match instruction {
            Instruction::RefreshPositions(positions) => positions.clone(),
            _ => return state,
        };

        let start = std::time::Instant::now();
        let position_count = positions.len();

        positions.retain(|position| has_crop_routes(&state, position));
        let free_positions = self
            .x
            .send_world(move |world| free_positions(world, positions))
            .await;

        let mut built: usize = 0;
        for position in free_positions {
            if self.build_crops(&mut state, &position).await {
                built += 1;
            }
        }

        info!(
            "Built {}/{} crops in {}ms",
            built,
            position_count,
            start.elapsed().as_millis()
        );

        state
    }
}

impl<X> TryBuildCrops<X>
where
    X: SendGame, // TOOD send routes
{
    pub fn new(x: X) -> TryBuildCrops<X> {
        TryBuildCrops { x }
    }

    async fn build_crops(&self, state: &mut State, position: &V2<usize>) -> bool {
        let mut routes = ok_or!(state.traffic.get(position), return false).clone();
        routes.retain(|route| route.resource == Resource::Crops);

        let routes: Vec<Route> = self
            .x
            .send_game(move |game| {
                routes
                    .into_iter()
                    .flat_map(|route_key| game.game_state().routes.get_route(&route_key))
                    .cloned()
                    .collect()
            })
            .await;

        let first_visit = routes
            .into_iter()
            .map(|route| route.start_micros + route.duration.as_micros())
            .min()
            .unwrap();

        state.build_queue.insert(BuildInstruction {
            what: Build::Crops {
                position: *position,
                rotated: true,
            }, // TODO random rotation
            when: first_visit,
        });

        true
    }
}

fn has_crop_routes(state: &State, position: &V2<usize>) -> bool {
    ok_or!(state.traffic.get(&position), return false)
        .iter()
        .any(|route| route.resource == Resource::Crops)
}

fn free_positions(world: &World, positions: HashSet<V2<usize>>) -> Vec<V2<usize>> {
    positions
        .into_iter()
        .filter(|position| is_free(world, position))
        .collect()
}

fn is_free(world: &World, position: &V2<usize>) -> bool {
    world
        .get_cell(&position)
        .map_or(false, |cell| cell.object == WorldObject::None)
}
