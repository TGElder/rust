use super::*;

use crate::settlement::*;
use serde::{Deserialize, Serialize};
use std::default::Default;

const HANDLE: &str = "homeland_population_sim";

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct HomelandPopulationSimParams {
    growth_rate: f32,
    max: usize,
}

impl Default for HomelandPopulationSimParams {
    fn default() -> HomelandPopulationSimParams {
        HomelandPopulationSimParams {
            growth_rate: 1.1,
            max: 65536,
        }
    }
}

pub struct HomelandPopulationSim {
    params: HomelandPopulationSimParams,
    game_tx: UpdateSender<Game>,
}

impl Step for HomelandPopulationSim {
    fn name(&self) -> &'static str {
        HANDLE
    }

    fn step(&mut self, _: u128) {
        block_on(self.step_async())
    }
}

impl HomelandPopulationSim {
    pub fn new(
        params: HomelandPopulationSimParams,
        game_tx: &UpdateSender<Game>,
    ) -> HomelandPopulationSim {
        HomelandPopulationSim {
            params,
            game_tx: game_tx.clone_with_handle(HANDLE),
        }
    }

    async fn step_async(&mut self) {
        let homelands = self.get_homelands().await;
        for homeland in homelands {
            self.grow_population(homeland).await
        }
    }

    async fn get_homelands(&mut self) -> Vec<V2<usize>> {
        self.game_tx.update(move |game| get_homelands(game)).await
    }

    async fn grow_population(&mut self, homeland: V2<usize>) {
        let growth_rate = self.params.growth_rate;
        let max = self.params.max;
        self.game_tx
            .update(move |game| grow_population(game, homeland, growth_rate, max))
            .await
    }
}

fn get_homelands(game: &mut Game) -> Vec<V2<usize>> {
    game.game_state()
        .settlements
        .values()
        .filter(|settlement| is_homeland(settlement))
        .map(|settlement| settlement.position)
        .collect()
}

fn is_homeland(settlement: &Settlement) -> bool {
    if let SettlementClass::Homeland = settlement.class {
        true
    } else {
        false
    }
}

fn grow_population(game: &mut Game, settlement: V2<usize>, growth_rate: f32, max: usize) {
    let settlement = match game.game_state().settlements.get(&settlement) {
        Some(settlement) => settlement,
        None => return,
    };
    let population = ((settlement.population as f32 * growth_rate) as usize).min(max);
    println!(
        "The population of {:?} increased from {} to {}",
        settlement.position, settlement.population, population
    );
    let updated_settlement = Settlement {
        population,
        ..*settlement
    };
    game.update_settlement(updated_settlement);
}
