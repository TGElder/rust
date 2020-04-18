use super::*;

use crate::game_event_consumers::FARM_CANDIDATE_TARGETS;
use commons::rand::prelude::*;
use commons::rand::rngs::SmallRng;

const HANDLE: &str = "farm_assigner_sim";

#[derive(Debug)]
struct Farmless {
    name: String,
    position: V2<usize>,
}

pub struct FarmAssignerSim {
    game_tx: UpdateSender<Game>,
    pathfinder_tx: UpdateSender<PathfinderService<AvatarTravelDuration>>,
    rng: SmallRng,
}

impl Step for FarmAssignerSim {
    fn name(&self) -> &'static str {
        HANDLE
    }

    fn step(&mut self, _: u128) {
        block_on(self.step_async())
    }
}

impl FarmAssignerSim {
    pub fn new(
        game_tx: &UpdateSender<Game>,
        pathfinder_tx: &UpdateSender<PathfinderService<AvatarTravelDuration>>,
        seed: u64,
    ) -> FarmAssignerSim {
        FarmAssignerSim {
            game_tx: game_tx.clone_with_handle(HANDLE),
            pathfinder_tx: pathfinder_tx.clone_with_handle(HANDLE),
            rng: SeedableRng::seed_from_u64(seed),
        }
    }

    async fn step_async(&mut self) {
        for farmless in self.get_farmless().await {
            self.step_farmless(farmless).await
        }
    }

    async fn get_farmless(&mut self) -> Vec<Farmless> {
        self.game_tx.update(get_farmless).await
    }

    async fn step_farmless(&mut self, farmless: Farmless) {
        let farm = match self.get_farm(farmless.position).await {
            Some(farm) => farm,
            None => return,
        };
        let rotated = self.rng.gen();
        if self.set_farm(farmless.name, farm, rotated).await {
            self.remove_candidate(farm).await;
        }
    }

    async fn get_farm(&mut self, position: V2<usize>) -> Option<V2<usize>> {
        self.pathfinder_tx
            .update(move |service| {
                let mut candidates = service
                    .pathfinder()
                    .closest_targets(&[position], FARM_CANDIDATE_TARGETS);
                candidates.pop().map(|result| result.position)
            })
            .await
    }

    async fn set_farm(&mut self, citizen: String, position: V2<usize>, rotated: bool) -> bool {
        self.game_tx
            .update(move |game| set_farm(game, citizen, position, rotated))
            .await
    }

    async fn remove_candidate(&mut self, farm: V2<usize>) {
        self.pathfinder_tx
            .update(move |service| {
                service
                    .pathfinder()
                    .load_target(FARM_CANDIDATE_TARGETS, &farm, false)
            })
            .await
    }
}

fn get_farmless(game: &mut Game) -> Vec<Farmless> {
    game.game_state()
        .citizens
        .values()
        .flat_map(as_farmless)
        .collect()
}

fn as_farmless(citizen: &Citizen) -> Option<Farmless> {
    if citizen.farm.is_some() {
        return None;
    }
    Some(Farmless {
        name: citizen.name.clone(),
        position: citizen.birthplace,
    })
}

fn set_farm(game: &mut Game, citizen: String, farm: V2<usize>, rotated: bool) -> bool {
    if !game.game_state().citizens.contains_key(&citizen) {
        return false;
    }
    if game.update_object(WorldObject::Farm { rotated }, farm, true) {
        let citizen = game.mut_state().citizens.get_mut(&citizen).unwrap();
        citizen.farm = Some(farm);
        true
    } else {
        false
    }
}
