use super::*;

use crate::route::*;
use commons::grid::*;
use commons::rand::prelude::*;
use commons::rand::rngs::SmallRng;
use std::collections::HashSet;

const HANDLE: &str = "crop_sim";

pub struct CropSim {
    game_tx: UpdateSender<Game>,
    rng: SmallRng,
}

impl Step for CropSim {
    fn name(&self) -> &'static str {
        HANDLE
    }

    fn init(&mut self) {}

    fn step(&mut self, _: u128) {
        block_on(self.step_async())
    }
}

impl CropSim {
    pub fn new(seed: u64, game_tx: &UpdateSender<Game>) -> CropSim {
        CropSim {
            game_tx: game_tx.clone_with_handle(HANDLE),
            rng: SeedableRng::seed_from_u64(seed),
        }
    }

    async fn step_async(&mut self) {
        let crops = self.get_crops().await;
        for crop in crops {
            self.build_crop(crop).await;
        }
    }

    async fn get_crops(&mut self) -> HashSet<V2<usize>> {
        self.game_tx.update(|game| get_crops(game)).await
    }

    async fn build_crop(&mut self, position: V2<usize>) {
        let rotated = self.rng.gen();
        self.game_tx
            .update(move |game| build_crop(game, position, rotated))
            .await
    }
}

fn get_crops(game: &mut Game) -> HashSet<V2<usize>> {
    game.game_state()
        .routes
        .values()
        .filter(|route| is_crops_route(route))
        .flat_map(|route| route.path.last())
        .filter(|position| !is_crop(game, position))
        .filter(|position| !is_town(game, position))
        .cloned()
        .collect()
}

fn is_crop(game: &Game, position: &V2<usize>) -> bool {
    match game.game_state().world.get_cell(position) {
        Some(WorldCell {
            object: WorldObject::Crop { .. },
            ..
        }) => true,
        _ => false,
    }
}

fn is_town(game: &Game, position: &V2<usize>) -> bool {
    game.game_state().settlements.contains_key(position)
}

fn is_crops_route(route: &Route) -> bool {
    if let Resource::Crops = route.resource {
        true
    } else {
        false
    }
}

fn build_crop(game: &mut Game, position: V2<usize>, rotated: bool) {
    game.add_object(WorldObject::Crop { rotated }, position);
}
