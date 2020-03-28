use super::*;

use crate::game_event_consumers::FARM_CANDIDATE_TARGETS;

const HANDLE: &str = "farm_assigner_sim";

#[derive(Debug)]
struct Farmless {
    name: String,
    position: V2<usize>,
}

pub struct FarmAssignerSim {
    game_tx: UpdateSender<Game>,
    pathfinder_tx: UpdateSender<Pathfinder<AvatarTravelDuration>>,
}

impl Step for FarmAssignerSim {
    fn step(&mut self, _: u128) {
        block_on(self.step_async())
    }
}

impl FarmAssignerSim {
    pub fn new(
        game_tx: &UpdateSender<Game>,
        pathfinder_tx: &UpdateSender<Pathfinder<AvatarTravelDuration>>,
    ) -> FarmAssignerSim {
        FarmAssignerSim {
            game_tx: game_tx.clone_with_handle(HANDLE),
            pathfinder_tx: pathfinder_tx.clone_with_handle(HANDLE),
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
        if self.set_farm(farmless.name, farm).await {
            self.remove_candidate(farm).await;
        }
    }

    async fn get_farm(&mut self, position: V2<usize>) -> Option<V2<usize>> {
        self.pathfinder_tx
            .update(move |pathfinder| {
                let mut candidates =
                    pathfinder.closest_targets(&[position], FARM_CANDIDATE_TARGETS);
                candidates.pop()
            })
            .await
    }

    async fn set_farm(&mut self, avatar: String, position: V2<usize>) -> bool {
        self.game_tx
            .update(move |game| set_farm(game, avatar, position))
            .await
    }

    async fn remove_candidate(&mut self, farm: V2<usize>) {
        self.pathfinder_tx
            .update(move |pathfinder| pathfinder.load_target(FARM_CANDIDATE_TARGETS, &farm, false))
            .await
    }
}

fn get_farmless(game: &mut Game) -> Vec<Farmless> {
    game.game_state()
        .avatars
        .values()
        .flat_map(as_farmless)
        .collect()
}

fn as_farmless(avatar: &Avatar) -> Option<Farmless> {
    if avatar.farm.is_some() {
        return None;
    }
    let position = match avatar.state {
        AvatarState::Stationary { position, .. } => position,
        _ => return None,
    };
    Some(Farmless {
        name: avatar.name.clone(),
        position,
    })
}

fn set_farm(game: &mut Game, avatar: String, farm: V2<usize>) -> bool {
    if !game.game_state().avatars.contains_key(&avatar) {
        return false;
    }
    if game.update_object(WorldObject::Farm, farm, true) {
        let avatar = game.mut_state().avatars.get_mut(&avatar).unwrap();
        avatar.farm = Some(farm);
        true
    } else {
        false
    }
}
