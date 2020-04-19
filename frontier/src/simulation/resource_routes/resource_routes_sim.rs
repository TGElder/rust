#![allow(clippy::trivially_copy_pass_by_ref)]

use super::resource_routes_targets::target_set;
use super::*;

use commons::grid::get_corners;
use std::collections::HashSet;

const HANDLE: &str = "resource_route_sim";
const ROUTE_PREFIX: &str = "resource-";

pub struct ResourceRouteSim {
    game_tx: UpdateSender<Game>,
    pathfinder_tx: UpdateSender<PathfinderService<AvatarTravelDuration>>,
}

impl Step for ResourceRouteSim {
    fn name(&self) -> &'static str {
        HANDLE
    }

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
        self.clear_routes().await;
        let towns = self.get_towns().await;
        for resource in RESOURCES.iter() {
            self.step_resource(&towns, resource).await;
        }
    }

    async fn clear_routes(&mut self) {
        self.game_tx.update(|game| clear_routes(game)).await;
    }

    async fn get_towns(&mut self) -> HashSet<V2<usize>> {
        self.game_tx.update(|game| get_towns(game)).await
    }

    async fn step_resource(&mut self, towns: &HashSet<V2<usize>>, resource: &Resource) {
        for town in towns.iter() {
            if let Some(path) = self.get_path_to_resource(*town, resource).await {
                self.add_route(town, resource, path).await;
            }
        }
    }

    async fn get_path_to_resource(
        &mut self,
        town: V2<usize>,
        resource: &Resource,
    ) -> Option<Vec<V2<usize>>> {
        let target_set = target_set(resource);
        self.pathfinder_tx
            .update(move |service| {
                get_path_to_resource(&mut service.pathfinder(), town, target_set)
            })
            .await
    }

    async fn add_route(&mut self, town: &V2<usize>, resource: &Resource, path: Vec<V2<usize>>) {
        let name = route_name(town, resource);
        self.game_tx
            .update(move |game| add_route(game, name, path))
            .await;
    }
}

fn clear_routes(game: &mut Game) {
    game.mut_state()
        .routes
        .retain(|route_name, _| !created_here(route_name));
}

fn created_here(route_name: &str) -> bool {
    route_name.starts_with(ROUTE_PREFIX)
}

fn get_towns(game: &mut Game) -> HashSet<V2<usize>> {
    game.game_state().territory.controllers()
}

fn get_path_to_resource(
    pathfinder: &mut Pathfinder<AvatarTravelDuration>,
    town: V2<usize>,
    target_set: String,
) -> Option<Vec<V2<usize>>> {
    pathfinder
        .closest_targets(&get_corners(&town), &target_set, 1)
        .pop()
        .map(|result| result.path)
}

fn add_route(game: &mut Game, name: String, path: Vec<V2<usize>>) {
    game.mut_state().routes.insert(name, path);
}

fn route_name(town: &V2<usize>, resource: &Resource) -> String {
    format!("{}-{:?}-{}", ROUTE_PREFIX, town, resource.name())
}
