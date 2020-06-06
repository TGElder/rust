use super::demand::*;
use super::resource_routes_targets::target_set;
use super::*;

use crate::route::*;
use commons::grid::get_corners;
use std::collections::HashMap;

const HANDLE: &str = "resource_route_sim";

pub struct ResourceRouteSim {
    game_tx: UpdateSender<Game>,
    pathfinder_tx: UpdateSender<PathfinderService<AvatarTravelDuration>>,
}

impl Step for ResourceRouteSim {
    fn name(&self) -> &'static str {
        HANDLE
    }

    fn init(&mut self) {}

    fn step(&mut self, _: u128) {
        block_on(self.step_async())
    }
}

impl ResourceRouteSim {
    pub fn new(
        game_tx: &UpdateSender<Game>,
        pathfinder_tx: &UpdateSender<PathfinderService<AvatarTravelDuration>>,
    ) -> ResourceRouteSim {
        ResourceRouteSim {
            game_tx: game_tx.clone_with_handle(HANDLE),
            pathfinder_tx: pathfinder_tx.clone_with_handle(HANDLE),
        }
    }

    async fn step_async(&mut self) {
        let mut routes = HashMap::new();
        let settlements = self.get_settlements().await;
        for settlement in settlements {
            routes.extend(self.step_settlement(&settlement).await);
        }
        println!("{} routes", routes.len());
        let game_micros = self.get_game_micros().await;
        self.update_routes_start_micros(&mut routes, game_micros);
        self.update_routes(routes).await;
    }

    async fn get_settlements(&mut self) -> Vec<Settlement> {
        self.game_tx.update(|game| get_settlements(game)).await
    }

    async fn step_settlement(&mut self, settlement: &Settlement) -> HashMap<String, Route> {
        let mut out = HashMap::new();
        for demand in get_demands(&settlement) {
            out.extend(self.create_routes_from_demand(settlement, demand).await)
        }
        out
    }

    async fn create_routes_from_demand(
        &mut self,
        settlement: &Settlement,
        demand: Demand,
    ) -> HashMap<String, Route> {
        let mut out = HashMap::new();
        let targets = self
            .get_closest_targets(settlement.position, demand.resource, demand.sources)
            .await;
        if targets.is_empty() {
            return out;
        }
        let mut traffic: Vec<usize> = vec![0; targets.len()];
        for i in 0..demand.sources {
            traffic[i % targets.len()] += 1
        }
        for i in 0..targets.len() {
            let target = &targets[i % targets.len()];
            out.extend(create_route(
                demand.resource,
                settlement.position,
                target.path.clone(),
                target.duration,
                traffic[i],
            ));
        }
        out
    }

    async fn get_closest_targets(
        &mut self,
        settlement: V2<usize>,
        resource: Resource,
        sources: usize,
    ) -> Vec<ClosestTargetResult> {
        let target_set = target_set(resource);
        self.pathfinder_tx
            .update(move |service| {
                get_closest_targets(&mut service.pathfinder(), settlement, target_set, sources)
            })
            .await
    }

    async fn get_game_micros(&mut self) -> u128 {
        self.game_tx.update(|game| get_game_micros(game)).await
    }

    fn update_routes_start_micros(
        &mut self,
        routes: &mut HashMap<String, Route>,
        start_micros: u128,
    ) {
        for route in routes.values_mut() {
            route.start_micros = start_micros;
        }
    }

    async fn update_routes(&mut self, routes: HashMap<String, Route>) {
        self.game_tx
            .update(|game| update_routes(game, routes))
            .await;
    }
}

fn get_settlements(game: &mut Game) -> Vec<Settlement> {
    game.game_state().settlements.values().cloned().collect()
}

fn get_closest_targets(
    pathfinder: &mut Pathfinder<AvatarTravelDuration>,
    settlement: V2<usize>,
    target_set: String,
    sources: usize,
) -> Vec<ClosestTargetResult> {
    let corners: Vec<V2<usize>> = get_corners(&settlement)
        .drain(..)
        .filter(|corner| pathfinder.in_bounds(corner))
        .collect();
    pathfinder.closest_targets(&corners, &target_set, sources)
}

fn create_route(
    resource: Resource,
    settlement: V2<usize>,
    path: Vec<V2<usize>>,
    duration: Duration,
    traffic: usize,
) -> Option<(String, Route)> {
    if let [from, .., to] = path.as_slice() {
        Some((
            route_name(resource, from, to),
            Route {
                resource,
                settlement,
                path,
                duration,
                start_micros: 0,
                traffic,
            },
        ))
    } else {
        None
    }
}

fn route_name(resource: Resource, from: &V2<usize>, to: &V2<usize>) -> String {
    format!("{}-{:?}-{:?}", resource.name(), from, to)
}

fn get_game_micros(game: &mut Game) -> u128 {
    game.game_state().game_micros
}

fn update_routes(game: &mut Game, routes: HashMap<String, Route>) {
    game.mut_state().routes = routes
}
