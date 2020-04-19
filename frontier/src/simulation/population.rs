use super::*;

use crate::settlement::*;
use std::collections::HashMap;

const HANDLE: &str = "population_sim";

pub struct PopulationSim {
    game_tx: UpdateSender<Game>,
}

impl Step for PopulationSim {
    fn name(&self) -> &'static str {
        HANDLE
    }

    fn step(&mut self, _: u128) {
        block_on(self.step_async())
    }
}

impl PopulationSim {
    pub fn new(game_tx: &UpdateSender<Game>) -> PopulationSim {
        PopulationSim {
            game_tx: game_tx.clone_with_handle(HANDLE),
        }
    }

    async fn step_async(&mut self) {
        let route_ends = self.get_route_ends().await;
        let controllers = self.get_controllers(route_ends).await;
        let populations = get_populations(controllers);
        self.set_populations(populations).await
    }

    async fn get_route_ends(&mut self) -> Vec<V2<usize>> {
        self.game_tx.update(move |game| get_route_ends(game)).await
    }

    async fn get_controllers(&mut self, positions: Vec<V2<usize>>) -> Vec<V2<usize>> {
        self.game_tx
            .update(move |game| get_controllers(game, positions))
            .await
    }

    async fn set_populations(&mut self, populations: HashMap<V2<usize>, usize>) {
        self.game_tx
            .update(move |game| set_populations(game, populations))
            .await
    }
}

fn get_route_ends(game: &mut Game) -> Vec<V2<usize>> {
    game.game_state()
        .routes
        .values()
        .flat_map(|route| route.path.last())
        .cloned()
        .collect()
}

fn get_controllers(game: &mut Game, positions: Vec<V2<usize>>) -> Vec<V2<usize>> {
    positions
        .iter()
        .flat_map(|position| game.game_state().territory.who_controls(position))
        .map(|claim| claim.controller)
        .collect()
}

fn get_populations(controllers: Vec<V2<usize>>) -> HashMap<V2<usize>, usize> {
    let mut out = HashMap::new();
    for controller in controllers {
        *out.entry(controller).or_insert(0) += 1
    }
    out
}

fn set_populations(game: &mut Game, populations: HashMap<V2<usize>, usize>) {
    let updated: Vec<Settlement> = game
        .game_state()
        .settlements
        .values()
        .filter(|settlement| is_town(settlement))
        .map(|settlement| Settlement {
            population: *populations.get(&settlement.position).unwrap_or(&0),
            ..*settlement
        })
        .collect();
    for settlement in updated {
        game.update_settlement(settlement);
    }
}

fn is_town(settlement: &Settlement) -> bool {
    if let SettlementClass::Town = settlement.class {
        true
    } else {
        false
    }
}
