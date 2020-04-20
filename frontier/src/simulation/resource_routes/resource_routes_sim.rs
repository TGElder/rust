use super::demand::get_demands;
use super::resource_routes_targets::target_set;
use super::*;

use crate::route::*;
use commons::grid::get_corners;

const HANDLE: &str = "resource_route_sim";

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
        let settlements = self.get_settlements().await;
        for settlement in settlements {
            self.step_settlement(&settlement).await;
        }
    }

    async fn clear_routes(&mut self) {
        self.game_tx.update(|game| clear_routes(game)).await;
    }

    async fn get_settlements(&mut self) -> Vec<Settlement> {
        self.game_tx.update(|game| get_settlements(game)).await
    }

    async fn step_settlement(&mut self, settlement: &Settlement) {
        for demand in get_demands(&settlement) {
            let mut paths = self
                .get_paths_to_resource(settlement.position, demand.resource, demand.sources)
                .await;
            for path in paths.drain(..) {
                self.add_routes(demand.resource, path, demand.quantity)
                    .await;
            }
        }
    }

    async fn get_paths_to_resource(
        &mut self,
        settlement: V2<usize>,
        resource: Resource,
        sources: usize,
    ) -> Vec<Vec<V2<usize>>> {
        let target_set = target_set(resource);
        self.pathfinder_tx
            .update(move |service| {
                get_paths_to_resource(&mut service.pathfinder(), settlement, target_set, sources)
            })
            .await
    }

    async fn add_routes(&mut self, resource: Resource, path: Vec<V2<usize>>, quantity: usize) {
        self.game_tx
            .update(move |game| add_routes(game, resource, path, quantity))
            .await;
    }
}

fn clear_routes(game: &mut Game) {
    game.mut_state().routes.clear();
}

fn get_settlements(game: &mut Game) -> Vec<Settlement> {
    game.game_state().settlements.values().cloned().collect()
}

fn get_paths_to_resource(
    pathfinder: &mut Pathfinder<AvatarTravelDuration>,
    settlement: V2<usize>,
    target_set: String,
    sources: usize,
) -> Vec<Vec<V2<usize>>> {
    let corners: Vec<V2<usize>> = get_corners(&settlement)
        .drain(..)
        .filter(|corner| pathfinder.in_bounds(corner))
        .collect();
    pathfinder
        .closest_targets(&corners, &target_set, sources)
        .drain(..)
        .map(|result| result.path)
        .collect()
}

fn add_routes(game: &mut Game, resource: Resource, path: Vec<V2<usize>>, quantity: usize) {
    for i in 0..quantity {
        let from = match path.first() {
            Some(first) => first,
            None => continue,
        };
        let to = match path.last() {
            Some(last) => last,
            None => continue,
        };
        let name = route_name(resource, from, to, i);
        let route = Route {
            resource,
            path: path.clone(),
        };
        game.mut_state().routes.insert(name, route);
    }
}

fn route_name(resource: Resource, from: &V2<usize>, to: &V2<usize>, number: usize) -> String {
    format!("{}-{}-{}-{}", resource.name(), from, to, number,)
}
