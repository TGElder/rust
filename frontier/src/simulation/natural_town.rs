use super::*;
use crate::nation::Nation;
use crate::route::*;
use crate::settlement::*;
use commons::grid::Grid;
use std::collections::{HashMap, HashSet};

const HANDLE: &str = "natural_town_sim";
const ROUTE_BATCH_SIZE: usize = 128;
const CANDIDATE_BATCH_SIZE: usize = 128;

pub struct NaturalTownSim {
    game_tx: UpdateSender<Game>,
    territory_sim: TerritorySim,
}

impl Step for NaturalTownSim {
    fn name(&self) -> &'static str {
        HANDLE
    }

    fn init(&mut self) {}

    fn step(&mut self, _: u128) {
        block_on(self.step_async())
    }
}

impl NaturalTownSim {
    pub fn new(game_tx: &UpdateSender<Game>, territory_sim: TerritorySim) -> NaturalTownSim {
        NaturalTownSim {
            game_tx: game_tx.clone_with_handle(HANDLE),
            territory_sim,
        }
    }

    async fn step_async(&mut self) {
        let routes = self.get_routes().await;
        self.update_first_visit(&routes).await;
        let mut visitors = self.compute_visitors(&routes).await;
        while let Some(town) = self.find_town_candidate(&visitors) {
            if self.build_town(town).await {
                self.territory_sim.step_controller(town).await;
            }
            visitors.remove(&town);
            visitors = self.remove_already_controlled(visitors).await;
        }
    }

    async fn get_routes(&mut self) -> Vec<String> {
        self.game_tx.update(|game| get_routes(game)).await
    }

    async fn update_first_visit(&mut self, routes: &[String]) {
        self.update_economic_activity_first_visits(routes).await;
        self.update_all_first_visits(routes).await;
    }

    async fn update_economic_activity_first_visits(&mut self, routes: &[String]) {
        for batch in routes.chunks(ROUTE_BATCH_SIZE) {
            self.update_economic_activity_first_visits_for_routes(batch.to_vec())
                .await;
        }
    }

    async fn update_all_first_visits(&mut self, routes: &[String]) {
        for batch in routes.chunks(ROUTE_BATCH_SIZE) {
            self.update_all_first_visits_for_routes(batch.to_vec())
                .await;
        }
    }

    async fn update_economic_activity_first_visits_for_routes(&mut self, routes: Vec<String>) {
        self.game_tx
            .update(move |game| update_economic_activity_first_visits_for_routes(game, routes))
            .await;
    }

    async fn update_all_first_visits_for_routes(&mut self, routes: Vec<String>) {
        self.game_tx
            .update(move |game| update_all_first_visits_for_routes(game, routes))
            .await;
    }

    async fn compute_visitors(&mut self, routes: &[String]) -> HashMap<V2<usize>, usize> {
        let mut out = HashMap::new();
        for batch in routes.chunks(ROUTE_BATCH_SIZE) {
            for (position, visitors) in self.compute_visitors_for_routes(batch.to_vec()).await {
                *out.entry(position).or_insert(0) += visitors;
            }
        }
        out
    }

    async fn compute_visitors_for_routes(
        &mut self,
        routes: Vec<String>,
    ) -> HashMap<V2<usize>, usize> {
        self.game_tx
            .update(move |game| compute_visitors_for_routes(game, routes))
            .await
    }

    fn find_town_candidate(&self, visitors: &HashMap<V2<usize>, usize>) -> Option<V2<usize>> {
        visitors
            .iter()
            .max_by(|a, b| a.1.cmp(&b.1))
            .map(|(position, _)| *position)
    }

    async fn build_town(&mut self, position: V2<usize>) -> bool {
        self.game_tx
            .update(move |game| build_town(game, position))
            .await
    }

    async fn remove_already_controlled(
        &mut self,
        mut visitors: HashMap<V2<usize>, usize>,
    ) -> HashMap<V2<usize>, usize> {
        let candidates: Vec<V2<usize>> = visitors.keys().cloned().collect();
        for batch in candidates.chunks(CANDIDATE_BATCH_SIZE) {
            for candidate in self.get_already_controlled(batch.to_vec()).await {
                visitors.remove(&candidate);
            }
        }
        visitors
    }

    async fn get_already_controlled(&mut self, candidates: Vec<V2<usize>>) -> HashSet<V2<usize>> {
        self.game_tx
            .update(move |game| get_already_controlled(game, candidates))
            .await
    }
}

fn get_routes(game: &Game) -> Vec<String> {
    game.game_state().routes.keys().cloned().collect()
}

fn update_economic_activity_first_visits_for_routes(game: &mut Game, routes: Vec<String>) {
    for route in routes {
        update_economic_activity_first_visits_for_route(game, route);
    }
}

fn update_all_first_visits_for_routes(game: &mut Game, routes: Vec<String>) {
    for route in routes {
        update_all_first_visits_for_route(game, route);
    }
}

fn update_economic_activity_first_visits_for_route(game: &mut Game, route: String) {
    let route = unwrap_or!(game.game_state().routes.get(&route), return);
    let first_visit = FirstVisit {
        when: route.start_micros + route.duration.as_micros(),
        who: Some(route.settlement),
    };
    let to_update: Vec<V2<usize>> = get_economic_activity_traffic(game, &route)
        .map(|Traffic { position, .. }| position)
        .collect();
    for position in to_update {
        update_first_visit_if_required(game, &position, first_visit);
    }
}

fn update_all_first_visits_for_route(game: &mut Game, route: String) {
    let route = unwrap_or!(game.game_state().routes.get(&route), return);
    let first_visit = FirstVisit {
        when: route.start_micros + route.duration.as_micros(),
        who: None,
    };
    let to_update = route.path.clone();
    for position in to_update {
        update_first_visit_if_required(game, &position, first_visit);
    }
}

fn update_first_visit_if_required(game: &mut Game, position: &V2<usize>, first_visit: FirstVisit) {
    let maybe_first_visit = ok_or!(game.mut_state().first_visits.get_mut(position), return);
    match maybe_first_visit {
        None => *maybe_first_visit = Some(first_visit),
        Some(FirstVisit {
            when: current_first_visit,
            ..
        }) if first_visit.when < *current_first_visit => *maybe_first_visit = Some(first_visit),
        _ => (),
    };
}

fn compute_visitors_for_routes(game: &Game, routes: Vec<String>) -> HashMap<V2<usize>, usize> {
    let game_state = game.game_state();
    routes
        .iter()
        .flat_map(|route| game_state.routes.get(route))
        .flat_map(|route| get_economic_activity_traffic(game, &route))
        .filter(|Traffic { position, .. }| !game_state.world.is_sea(position))
        .filter(|Traffic { position, .. }| visited(game_state, position))
        .filter(|Traffic { position, .. }| all_corners_visible(&game_state, &position))
        .filter(|Traffic { position, .. }| !already_controlled(&game_state, &position))
        .fold(HashMap::new(), |mut map, traffic| {
            *map.entry(traffic.position).or_insert(0) += traffic.traffic;
            map
        })
}

fn get_economic_activity_traffic<'a>(
    game: &'a Game,
    route: &'a Route,
) -> impl Iterator<Item = Traffic> + 'a {
    get_destination_traffic(game, route).chain(get_port_traffic(game, &route))
}

fn get_destination_traffic<'a>(
    game: &'a Game,
    route: &'a Route,
) -> Box<dyn Iterator<Item = Traffic> + 'a> {
    let end = unwrap_or!(route.path.last(), return Box::new(std::iter::empty()));
    Box::new(
        game.game_state()
            .world
            .get_adjacent_tiles_in_bounds(&end)
            .into_iter()
            .map(move |position| Traffic {
                position,
                traffic: route.traffic,
            }),
    )
}

fn get_port_traffic<'a>(game: &'a Game, route: &'a Route) -> impl Iterator<Item = Traffic> + 'a {
    get_port_positions(game, &route.path)
        .flat_map(move |position| {
            game.game_state()
                .world
                .get_adjacent_tiles_in_bounds(&position)
        })
        .map(move |position| Traffic {
            position,
            traffic: route.traffic,
        })
}

fn already_controlled(game_state: &GameState, position: &V2<usize>) -> bool {
    game_state.territory.anyone_controls_tile(position)
}

fn all_corners_visible(game_state: &GameState, position: &V2<usize>) -> bool {
    let world = &game_state.world;
    world
        .get_corners_in_bounds(position)
        .iter()
        .flat_map(|position| world.get_cell(position))
        .all(|corner| corner.visible)
}

fn build_town(game: &mut Game, position: V2<usize>) -> bool {
    let nation = unwrap_or!(get_first_visit_nation(game, position), return false);
    let name = get_nation(game, &nation).get_town_name();
    let settlement = Settlement {
        class: SettlementClass::Town,
        position,
        name,
        nation,
        current_population: 0.0,
        target_population: 0.0,
        gap_half_life: None,
    };
    game.add_settlement(settlement)
}

fn get_nation<'a>(game: &'a mut Game, name: &'a str) -> &'a mut Nation {
    game.mut_state()
        .nations
        .get_mut(name)
        .unwrap_or_else(|| panic!("Unknown nation {}", name))
}

fn get_first_visit_nation(game: &Game, position: V2<usize>) -> Option<String> {
    let maybe_first_visit = unwrap_or!(
        game.game_state().first_visits.get_cell(&position),
        return None
    );
    let maybe_parent_position = unwrap_or!(maybe_first_visit, return None).who;
    let parent_position = unwrap_or!(maybe_parent_position, return None);
    game.game_state()
        .settlements
        .get(&parent_position)
        .map(|settlement| settlement.nation.clone())
}

fn get_already_controlled(game: &mut Game, mut candidates: Vec<V2<usize>>) -> HashSet<V2<usize>> {
    candidates
        .drain(..)
        .filter(|position| already_controlled(game.game_state(), position))
        .collect()
}

struct Traffic {
    position: V2<usize>,
    traffic: usize,
}
