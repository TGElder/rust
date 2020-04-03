use super::*;

use commons::*;

const HANDLE: &str = "commuter_sim";

pub struct CommuterSim {
    game_tx: UpdateSender<Game>,
    pathfinder_tx: UpdateSender<PathfinderService<AvatarTravelDuration>>,
}

struct Commute {
    from: V2<usize>,
    to: V2<usize>,
}

impl Step for CommuterSim {
    fn step(&mut self, _: u128) {
        block_on(self.step_async())
    }
}

impl CommuterSim {
    pub fn new(
        game_tx: &UpdateSender<Game>,
        pathfinder_tx: &UpdateSender<PathfinderService<AvatarTravelDuration>>,
    ) -> CommuterSim {
        CommuterSim {
            game_tx: game_tx.clone_with_handle(HANDLE),
            pathfinder_tx: pathfinder_tx.clone_with_handle(HANDLE),
        }
    }

    async fn step_async(&mut self) {
        for avatar in self.get_avatars().await {
            self.step_avatar(avatar).await
        }
    }

    async fn get_avatars(&mut self) -> Vec<String> {
        self.game_tx
            .update(|game| game.game_state().avatars.keys().cloned().collect())
            .await
    }

    async fn step_avatar(&mut self, avatar: String) {
        let commute = match self.get_commute(avatar.clone()).await {
            Some(commute) => commute,
            None => return,
        };
        let path = self.get_path(commute).await;
        self.set_commute(avatar, path).await;
    }

    async fn get_commute(&mut self, avatar: String) -> Option<Commute> {
        self.game_tx
            .update(move |game| get_commute(game, avatar))
            .await
    }

    async fn get_path(&mut self, commute: Commute) -> Option<Vec<V2<usize>>> {
        self.pathfinder_tx
            .update(move |service| get_path(&mut service.pathfinder(), commute))
            .await
    }

    async fn set_commute(&mut self, avatar: String, commute: Option<Vec<V2<usize>>>) {
        self.game_tx
            .update(move |game| set_commute(game, avatar, commute))
            .await
    }
}

fn get_commute(game: &mut Game, avatar: String) -> Option<Commute> {
    let game_state = game.game_state();
    let avatar = match game_state.avatars.get(&avatar) {
        Some(avatar) => avatar,
        _ => return None,
    };
    let to = match avatar.farm {
        Some(to) => to,
        _ => return None,
    };
    game_state
        .territory
        .who_controls_tile(&to)
        .map(|claim| claim.controller)
        .map(|from| Commute { to, from })
}

fn get_path(
    pathfinder: &mut Pathfinder<AvatarTravelDuration>,
    commute: Commute,
) -> Option<Vec<V2<usize>>> {
    pathfinder.find_path(&get_corners(&commute.from), &get_corners(&commute.to))
}

fn set_commute(game: &mut Game, avatar: String, commute: Option<Vec<V2<usize>>>) {
    if let Some(avatar) = game.mut_state().avatars.get_mut(&avatar) {
        avatar.commute = commute;
    };
}
