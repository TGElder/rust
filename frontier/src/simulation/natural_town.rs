use super::*;
use crate::settlement::*;
use isometric::Color;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::default::Default;

const HANDLE: &str = "natural_town_sim";
const ROUTE_BATCH_SIZE: usize = 128;
const CANDIDATE_BATCH_SIZE: usize = 128;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct NaturalTownSimParams {
    visitor_count_threshold: usize,
}

impl Default for NaturalTownSimParams {
    fn default() -> NaturalTownSimParams {
        NaturalTownSimParams {
            visitor_count_threshold: 1,
        }
    }
}

pub struct NaturalTownSim {
    params: NaturalTownSimParams,
    town_color: Color,
    game_tx: UpdateSender<Game>,
    territory_sim: TerritorySim,
}

impl Step for NaturalTownSim {
    fn name(&self) -> &'static str {
        HANDLE
    }

    fn step(&mut self, _: u128) {
        block_on(self.step_async())
    }
}

impl NaturalTownSim {
    pub fn new(
        params: NaturalTownSimParams,
        town_color: Color,
        game_tx: &UpdateSender<Game>,
        territory_sim: TerritorySim,
    ) -> NaturalTownSim {
        NaturalTownSim {
            params,
            town_color,
            game_tx: game_tx.clone_with_handle(HANDLE),
            territory_sim,
        }
    }

    async fn step_async(&mut self) {
        let mut visitors = self.compute_visitors().await;
        visitors = self.filter_over_threshold(visitors);
        while let Some(town) = self.find_town_candidate(&visitors) {
            if self.build_town(town).await {
                self.territory_sim.step_controller(town).await;
            }
            visitors.remove(&town);
            visitors = self.remove_non_candidates(visitors).await;
        }
    }

    async fn compute_visitors(&mut self) -> HashMap<V2<usize>, usize> {
        let mut out = HashMap::new();
        let routes = self.get_routes().await;
        for batch in routes.chunks(ROUTE_BATCH_SIZE) {
            for (position, visitors) in self.compute_visitors_for_routes(batch.to_vec()).await {
                *out.entry(position).or_insert(0) += visitors;
            }
        }
        out
    }

    async fn get_routes(&mut self) -> Vec<String> {
        self.game_tx.update(|game| get_routes(game)).await
    }

    async fn compute_visitors_for_routes(
        &mut self,
        routes: Vec<String>,
    ) -> HashMap<V2<usize>, usize> {
        self.game_tx
            .update(move |game| compute_visitors_for_routes(game, routes))
            .await
    }

    fn filter_over_threshold(
        &self,
        mut visitors: HashMap<V2<usize>, usize>,
    ) -> HashMap<V2<usize>, usize> {
        let threshold = self.params.visitor_count_threshold;
        visitors
            .drain()
            .filter(|(_, visitors)| *visitors >= threshold)
            .collect()
    }

    fn find_town_candidate(&self, visitors: &HashMap<V2<usize>, usize>) -> Option<V2<usize>> {
        visitors
            .iter()
            .max_by(|a, b| a.1.cmp(&b.1))
            .map(|(position, _)| *position)
    }

    async fn build_town(&mut self, position: V2<usize>) -> bool {
        let town_color = self.town_color;
        self.game_tx
            .update(move |game| build_town(game, position, town_color))
            .await
    }

    async fn remove_non_candidates(
        &mut self,
        mut visitors: HashMap<V2<usize>, usize>,
    ) -> HashMap<V2<usize>, usize> {
        let candidates: Vec<V2<usize>> = visitors.keys().cloned().collect();
        for batch in candidates.chunks(CANDIDATE_BATCH_SIZE) {
            for candidate in self.find_non_candidates(batch.to_vec()).await {
                visitors.remove(&candidate);
            }
        }
        visitors
    }

    async fn find_non_candidates(&mut self, candidates: Vec<V2<usize>>) -> HashSet<V2<usize>> {
        self.game_tx
            .update(move |game| find_non_candidates(game, candidates))
            .await
    }
}

fn get_routes(game: &Game) -> Vec<String> {
    game.game_state().routes.keys().cloned().collect()
}

fn compute_visitors_for_routes(game: &Game, routes: Vec<String>) -> HashMap<V2<usize>, usize> {
    let game_state = game.game_state();
    routes
        .iter()
        .flat_map(|route| game_state.routes.get(route))
        .flat_map(|route| route.path.last())
        .filter(|position| is_town_candidate(&game_state, &position))
        .fold(HashMap::new(), |mut map, position| {
            *map.entry(*position).or_insert(0) += 1;
            map
        })
}

fn is_town_candidate(game_state: &GameState, position: &V2<usize>) -> bool {
    !game_state.world.is_sea(position) && !game_state.territory.anyone_controls_tile(position)
}

#[allow(clippy::collapsible_if)]
fn build_town(game: &mut Game, position: V2<usize>, color: Color) -> bool {
    let settlement = Settlement {
        class: SettlementClass::Town,
        position,
        color,
        population: 0,
    };
    game.add_settlement(settlement)
}

fn find_non_candidates(game: &mut Game, mut candidates: Vec<V2<usize>>) -> HashSet<V2<usize>> {
    candidates
        .drain(..)
        .filter(|position| !is_town_candidate(game.game_state(), position))
        .collect()
}
