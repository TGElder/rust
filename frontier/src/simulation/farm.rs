use super::*;

use crate::route::*;
use commons::grid::*;
use commons::rand::prelude::*;
use commons::rand::rngs::SmallRng;
use std::collections::HashSet;

const HANDLE: &str = "farm_sim";

pub struct FarmSim {
    game_tx: UpdateSender<Game>,
    rng: SmallRng,
}

impl Step for FarmSim {
    fn name(&self) -> &'static str {
        HANDLE
    }

    fn step(&mut self, _: u128) {
        block_on(self.step_async())
    }
}

impl FarmSim {
    pub fn new(seed: u64, game_tx: &UpdateSender<Game>) -> FarmSim {
        FarmSim {
            game_tx: game_tx.clone_with_handle(HANDLE),
            rng: SeedableRng::seed_from_u64(seed),
        }
    }

    async fn step_async(&mut self) {
        let farms = self.get_farms().await;
        for farm in farms {
            self.build_farm(farm).await;
        }
    }

    async fn get_farms(&mut self) -> HashSet<V2<usize>> {
        self.game_tx.update(|game| get_farms(game)).await
    }

    async fn build_farm(&mut self, position: V2<usize>) {
        let rotated = self.rng.gen();
        self.game_tx
            .update(move |game| build_farm(game, position, rotated))
            .await
    }
}

fn get_farms(game: &mut Game) -> HashSet<V2<usize>> {
    game.game_state()
        .routes
        .values()
        .filter(|route| is_farmland_route(route))
        .flat_map(|route| route.path.last())
        .filter(|position| !is_farm(game, position))
        .cloned()
        .collect()
}

fn is_farm(game: &Game, position: &V2<usize>) -> bool {
    match game.game_state().world.get_cell(position) {
        Some(WorldCell {
            object: WorldObject::Farm { .. },
            ..
        }) => true,
        _ => false,
    }
}

fn is_farmland_route(route: &Route) -> bool {
    if let Resource::Farmland = route.resource {
        true
    } else {
        false
    }
}

fn build_farm(game: &mut Game, position: V2<usize>, rotated: bool) {
    game.add_object(WorldObject::Farm { rotated }, position);
}
