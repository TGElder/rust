use super::*;

use commons::rand::prelude::*;
use commons::rand::rngs::SmallRng;
use commons::*;
use serde::{Deserialize, Serialize};
use std::default::Default;

const HANDLE: &str = "route_sim";
const JOURNEY_PREFIX: &str = "journey_";
const COMMUTE_PREFIX: &str = "commute_";

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
struct RouteCitizen {
    name: String,
    farm: V2<usize>,
}

struct Route {
    name: String,
    from: V2<usize>,
    to: V2<usize>,
}

impl Step for RouteSim {
    fn name(&self) -> &'static str {
        HANDLE
    }

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
        self.clear_routes().await;
        let citizens = self.get_citizens().await;
        for citizen in citizens.iter() {
            self.step_citizen(citizen, &citizens).await
        }
    }

    async fn clear_routes(&mut self) {
        self.game_tx.update(|game| clear_routes(game)).await
    }

    async fn get_citizens(&mut self) -> Vec<RouteCitizen> {
        self.game_tx.update(|game| get_citizens(game)).await
    }

    async fn step_citizen(&mut self, citizen: &RouteCitizen, citizens: &[RouteCitizen]) {
        let route = match self.get_route(citizen, citizens).await {
            Some(route) => route,
            None => return,
        };
        let name = route.name.clone();
        let path = match self.get_path(route).await {
            Some(path) => path,
            None => return,
        };
        self.add_route(name, path).await;
    }

    async fn get_route(
        &mut self,
        citizen: &RouteCitizen,
        citizens: &[RouteCitizen],
    ) -> Option<Route> {
        if self.go_on_journey() {
            self.get_journey(citizen, citizens)
        } else {
            self.get_commute(citizen.clone()).await
        }
    }

    fn go_on_journey(&mut self) -> bool {
        self.rng.gen_range(0.0, 1.0) <= self.params.journey_percentage
    }

    fn get_journey(&mut self, citizen: &RouteCitizen, citizens: &[RouteCitizen]) -> Option<Route> {
        let visiting = citizens.choose(&mut self.rng);
        visiting.map(|visiting| Route {
            name: journey_name(&citizen.name),
            from: citizen.farm,
            to: visiting.farm,
        })
    }

    async fn get_commute(&mut self, citizen: RouteCitizen) -> Option<Route> {
        self.game_tx
            .update(move |game| get_commute(game, citizen))
            .await
    }

    async fn get_path(&mut self, route: Route) -> Option<Vec<V2<usize>>> {
        self.pathfinder_tx
            .update(move |service| get_path(&mut service.pathfinder(), route))
            .await
    }

    async fn add_route(&mut self, name: String, route: Vec<V2<usize>>) {
        self.game_tx
            .update(move |game| add_route(game, name, route))
            .await
    }
}

fn clear_routes(game: &mut Game) {
    game.mut_state().routes.clear();
}

fn get_citizens(game: &mut Game) -> Vec<RouteCitizen> {
    game.game_state()
        .citizens
        .values()
        .flat_map(|citizen| as_route_citizen(citizen))
        .collect()
}

fn as_route_citizen(citizen: &Citizen) -> Option<RouteCitizen> {
    citizen.farm.map(|farm| RouteCitizen {
        name: citizen.name.clone(),
        farm,
    })
}

fn get_commute(game: &mut Game, citizen: RouteCitizen) -> Option<Route> {
    let game_state = game.game_state();
    game_state
        .territory
        .who_controls_tile(&citizen.farm)
        .map(|claim| claim.controller)
        .map(|from| Route {
            name: commute_name(&citizen.name),
            to: citizen.farm,
            from,
        })
}

fn journey_name(citizen_name: &str) -> String {
    format!("{:?}{:?}", JOURNEY_PREFIX, citizen_name)
}

fn commute_name(citizen_name: &str) -> String {
    format!("{:?}{:?}", COMMUTE_PREFIX, citizen_name)
}

fn get_path(
    pathfinder: &mut Pathfinder<AvatarTravelDuration>,
    route: Route,
) -> Option<Vec<V2<usize>>> {
    pathfinder.find_path(&get_corners(&route.from), &get_corners(&route.to))
}

fn add_route(game: &mut Game, name: String, route: Vec<V2<usize>>) {
    game.mut_state().routes.insert(name, route);
}
