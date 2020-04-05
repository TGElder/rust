use super::*;
use isometric::Color;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::default::Default;

const HANDLE: &str = "natural_town_sim";

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
            visitors = self.filter_to_candidates(visitors).await;
        }
    }

    async fn compute_visitors(&mut self) -> HashMap<V2<usize>, usize> {
        self.game_tx
            .update(move |game| compute_visitors(game))
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

    async fn filter_to_candidates(
        &mut self,
        visitors: HashMap<V2<usize>, usize>,
    ) -> HashMap<V2<usize>, usize> {
        self.game_tx
            .update(move |game| filter_to_candidates(game, visitors))
            .await
    }
}

fn compute_visitors(game: &Game) -> HashMap<V2<usize>, usize> {
    let mut out = HashMap::new();
    let game_state = game.game_state();
    for avatar in game_state.avatars.values() {
        if let Some(route) = &avatar.route {
            for position in route {
                if is_town_candidate(&game_state, &position) {
                    let visitors = out.entry(*position).or_insert(0);
                    *visitors += 1;
                }
            }
        }
    }
    out
}

fn is_town_candidate(game_state: &GameState, position: &V2<usize>) -> bool {
    if game_state.world.is_sea(position) {
        return false;
    }
    if let Some(Claim { duration, .. }) = game_state.territory.who_controls(position) {
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

fn filter_to_candidates(
    game: &mut Game,
    mut visitors: HashMap<V2<usize>, usize>,
) -> HashMap<V2<usize>, usize> {
    visitors
        .drain()
        .filter(|(position, _)| is_town_candidate(game.game_state(), position))
        .collect()
}
