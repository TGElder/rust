use super::*;
use commons::grid::Grid;
use isometric::Color;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::default::Default;

const HANDLE: &str = "natural_town_sim";
const AVATAR_BATCH_SIZE: usize = 128;
const CANDIDATE_BATCH_SIZE: usize = 128;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct NaturalTownSimParams {
    visitor_count_threshold: usize,
}

impl Default for NaturalTownSimParams {
    fn default() -> NaturalTownSimParams {
        NaturalTownSimParams {
            visitor_count_threshold: 32,
        }
    }
}

pub struct NaturalTownSim {
    params: NaturalTownSimParams,
    house_color: Color,
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
        house_color: Color,
        game_tx: &UpdateSender<Game>,
        territory_sim: TerritorySim,
    ) -> NaturalTownSim {
        NaturalTownSim {
            params,
            house_color,
            game_tx: game_tx.clone_with_handle(HANDLE),
            territory_sim,
        }
    }

    async fn step_async(&mut self) {
        let mut visitors = self.compute_visitors().await;
        visitors = self.filter_over_threshold(visitors);
        while let Some(town) = self.find_town_candidate(&visitors) {
            if self.build_town(town).await {
                self.territory_sim.step_controller(town).await
            }
            visitors.remove(&town);
            visitors = self.remove_non_candidates(visitors).await;
        }
    }

    async fn compute_visitors(&mut self) -> HashMap<V2<usize>, usize> {
        let mut out = HashMap::new();
        let avatars = self.get_avatars().await;
        for batch in avatars.chunks(AVATAR_BATCH_SIZE) {
            for (position, visitors) in self.compute_visitors_for_avatars(batch.to_vec()).await {
                *out.entry(position).or_insert(0) += visitors;
            }
        }
        out
    }

    async fn get_avatars(&mut self) -> Vec<String> {
        self.game_tx.update(|game| get_avatars(game)).await
    }

    async fn compute_visitors_for_avatars(
        &mut self,
        avatars: Vec<String>,
    ) -> HashMap<V2<usize>, usize> {
        self.game_tx
            .update(move |game| compute_visitors_for_avatars(game, avatars))
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
        let house_color = self.house_color;
        self.game_tx
            .update(move |game| build_town(game, position, house_color))
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

fn get_avatars(game: &Game) -> Vec<String> {
    game.game_state().avatars.keys().cloned().collect()
}

fn compute_visitors_for_avatars(game: &Game, avatars: Vec<String>) -> HashMap<V2<usize>, usize> {
    let game_state = game.game_state();
    avatars
        .iter()
        .flat_map(|avatar| game_state.avatars.get(avatar))
        .flat_map(|avatar| &avatar.route)
        .flat_map(|route| route.iter())
        .flat_map(|position| game_state.world.get_corners_behind_in_bounds(position))
        .filter(|tile| is_town_candidate(&game_state, &tile))
        .fold(HashMap::new(), |mut map, tile| {
            *map.entry(tile).or_insert(0) += 1;
            map
        })
}

fn is_town_candidate(game_state: &GameState, position: &V2<usize>) -> bool {
    if game_state.world.is_sea(position) {
        return false;
    }
    if let Some(WorldCell {
        object: WorldObject::Farm { .. },
        ..
    }) = game_state.world.get_cell(position)
    {
    } else {
        return false;
    }
    if let Some(Claim { duration, .. }) = game_state.territory.who_controls_tile(position) {
        if *duration <= game_state.params.town_exclusive_duration {
            return false;
        }
    }
    true
}

#[allow(clippy::collapsible_if)]
fn build_town(game: &mut Game, position: V2<usize>, house_color: Color) -> bool {
    let house = WorldObject::House(house_color);
    if game.clear_object(position) {
        game.update_object(house, position, true)
    } else {
        false
    }
}

fn find_non_candidates(game: &mut Game, mut candidates: Vec<V2<usize>>) -> HashSet<V2<usize>> {
    candidates
        .drain(..)
        .filter(|position| !is_town_candidate(game.game_state(), position))
        .collect()
}
