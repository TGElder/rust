use super::*;

use crate::route::*;
use crate::settlement::*;
use std::collections::HashMap;

const HANDLE: &str = "population_sim";

pub struct RouteSummary {
    start: V2<usize>,
    end: V2<usize>,
}

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
        let route_summaries = self.get_route_summaries().await;
        let activity = self.get_activity(route_summaries).await;
        let populations = get_populations(activity);
        let towns = self.get_towns().await;
        for town in towns {
            self.set_population(town, *populations.get(&town).unwrap_or(&0))
                .await
        }
    }

    async fn get_route_summaries(&mut self) -> Vec<RouteSummary> {
        self.game_tx
            .update(move |game| get_route_summaries(game))
            .await
    }

    async fn get_activity(&mut self, summaries: Vec<RouteSummary>) -> Vec<V2<usize>> {
        self.game_tx
            .update(move |game| get_activity(game, summaries))
            .await
    }

    async fn get_towns(&mut self) -> Vec<V2<usize>> {
        self.game_tx.update(move |game| get_towns(game)).await
    }

    async fn set_population(&mut self, town: V2<usize>, population: usize) {
        self.game_tx
            .update(move |game| set_population(game, town, population))
            .await
    }
}

fn get_route_summaries(game: &mut Game) -> Vec<RouteSummary> {
    game.game_state()
        .routes
        .values()
        .flat_map(|route| as_route_summary(route))
        .collect()
}

fn as_route_summary(route: &Route) -> Option<RouteSummary> {
    let path = &route.path;
    let start = match path.first() {
        Some(&start) => start,
        None => return None,
    };
    let end = match path.last() {
        Some(&end) => end,
        None => return None,
    };
    Some(RouteSummary { start, end })
}

fn get_activity(game: &mut Game, summaries: Vec<RouteSummary>) -> Vec<V2<usize>> {
    summaries
        .iter()
        .filter(|summary| is_activity(game, summary))
        .map(|summary| summary.end)
        .collect()
}

fn is_activity(game: &mut Game, summary: &RouteSummary) -> bool {
    let end_controller = match game.game_state().territory.who_controls(&summary.end) {
        Some(controller) => Some(controller),
        None => return false,
    };
    let start_controller = game.game_state().territory.who_controls(&summary.start);
    start_controller != end_controller
}

fn get_populations(activity: Vec<V2<usize>>) -> HashMap<V2<usize>, usize> {
    let mut out = HashMap::new();
    for position in activity {
        *out.entry(position).or_insert(0) += 1
    }
    out
}

fn get_towns(game: &mut Game) -> Vec<V2<usize>> {
    game.game_state()
        .settlements
        .values()
        .filter(|settlement| is_town(settlement))
        .map(|settlement| settlement.position)
        .collect()
}

fn set_population(game: &mut Game, settlement: V2<usize>, population: usize) {
    let settlement = match game.game_state().settlements.get(&settlement) {
        Some(settlement) => settlement,
        None => return,
    };
    if settlement.population != population {
        println!(
            "The population of {:?} increases from {} to {}",
            settlement.position, settlement.population, population
        );
        let updated_settlement = Settlement {
            population,
            ..*settlement
        };
        game.update_settlement(updated_settlement);
    }
}

fn is_town(settlement: &Settlement) -> bool {
    if let SettlementClass::Town = settlement.class {
        true
    } else {
        false
    }
}
