use super::*;

use commons::rand::prelude::*;
use commons::rand::rngs::SmallRng;
use commons::*;
use serde::{Deserialize, Serialize};
use std::default::Default;

const HANDLE: &str = "route_sim";

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct RouteSimParams {
    journey_percentage: f32,
}

impl Default for RouteSimParams {
    fn default() -> RouteSimParams {
        RouteSimParams {
            journey_percentage: 0.1,
        }
    }
}

pub struct RouteSim {
    params: RouteSimParams,
    rng: SmallRng,
    game_tx: UpdateSender<Game>,
    pathfinder_tx: UpdateSender<PathfinderService<AvatarTravelDuration>>,
}

#[derive(Clone)]
struct RouteAvatar {
    name: String,
    farm: V2<usize>,
}

struct Route {
    from: V2<usize>,
    to: V2<usize>,
}

impl Step for RouteSim {
    fn step(&mut self, _: u128) {
        block_on(self.step_async())
    }
}

impl RouteSim {
    pub fn new(
        params: RouteSimParams,
        seed: u64,
        game_tx: &UpdateSender<Game>,
        pathfinder_tx: &UpdateSender<PathfinderService<AvatarTravelDuration>>,
    ) -> RouteSim {
        RouteSim {
            params,
            rng: SeedableRng::seed_from_u64(seed),
            game_tx: game_tx.clone_with_handle(HANDLE),
            pathfinder_tx: pathfinder_tx.clone_with_handle(HANDLE),
        }
    }

    async fn step_async(&mut self) {
        let avatars = self.get_avatars().await;
        for avatar in avatars.iter() {
            self.step_avatar(avatar, &avatars).await
        }
    }

    async fn get_avatars(&mut self) -> Vec<RouteAvatar> {
        self.game_tx.update(|game| get_avatars(game)).await
    }

    async fn step_avatar(&mut self, avatar: &RouteAvatar, avatars: &[RouteAvatar]) {
        let route = match self.get_route(avatar, avatars).await {
            Some(route) => route,
            None => return,
        };
        let path = self.get_path(route).await;
        self.set_route(avatar.name.clone(), path).await;
    }

    async fn get_route(&mut self, avatar: &RouteAvatar, avatars: &[RouteAvatar]) -> Option<Route> {
        if self.go_on_journey() {
            self.get_journey(avatar, avatars)
        } else {
            self.get_commute(avatar.clone()).await
        }
    }

    fn go_on_journey(&mut self) -> bool {
        self.rng.gen_range(0.0, 1.0) <= self.params.journey_percentage
    }

    fn get_journey(&mut self, avatar: &RouteAvatar, avatars: &[RouteAvatar]) -> Option<Route> {
        let visiting = avatars.choose(&mut self.rng);
        visiting.map(|visiting| Route {
            from: avatar.farm,
            to: visiting.farm,
        })
    }

    async fn get_commute(&mut self, avatar: RouteAvatar) -> Option<Route> {
        self.game_tx
            .update(move |game| get_commute(game, avatar))
            .await
    }

    async fn get_path(&mut self, route: Route) -> Option<Vec<V2<usize>>> {
        self.pathfinder_tx
            .update(move |service| get_path(&mut service.pathfinder(), route))
            .await
    }

    async fn set_route(&mut self, avatar: String, route: Option<Vec<V2<usize>>>) {
        self.game_tx
            .update(move |game| set_route(game, avatar, route))
            .await
    }
}

fn get_avatars(game: &mut Game) -> Vec<RouteAvatar> {
    game.game_state()
        .avatars
        .values()
        .flat_map(|avatar| as_route_avatar(avatar))
        .collect()
}

fn as_route_avatar(avatar: &Avatar) -> Option<RouteAvatar> {
    avatar.farm.map(|farm| RouteAvatar {
        name: avatar.name.clone(),
        farm,
    })
}

fn get_commute(game: &mut Game, avatar: RouteAvatar) -> Option<Route> {
    let game_state = game.game_state();
    game_state
        .territory
        .who_controls_tile(&avatar.farm)
        .map(|claim| claim.controller)
        .map(|from| Route {
            to: avatar.farm,
            from,
        })
}

fn get_path(
    pathfinder: &mut Pathfinder<AvatarTravelDuration>,
    route: Route,
) -> Option<Vec<V2<usize>>> {
    pathfinder.find_path(&get_corners(&route.from), &get_corners(&route.to))
}

fn set_route(game: &mut Game, avatar: String, route: Option<Vec<V2<usize>>>) {
    if let Some(avatar) = game.mut_state().avatars.get_mut(&avatar) {
        avatar.route = route;
    };
}
