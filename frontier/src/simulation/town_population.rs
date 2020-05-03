use super::*;

use crate::route::*;
use crate::settlement::*;
use crate::territory::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::default::Default;

const HANDLE: &str = "town_population_sim";
const BATCH_SIZE: usize = 128;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct TownPopulationSimParams {
    population_per_traffic: f64,
}

impl Default for TownPopulationSimParams {
    fn default() -> TownPopulationSimParams {
        TownPopulationSimParams {
            population_per_traffic: 0.5,
        }
    }
}

pub struct TownPopulationSim {
    params: TownPopulationSimParams,
    game_tx: UpdateSender<Game>,
}

impl Step for TownPopulationSim {
    fn name(&self) -> &'static str {
        HANDLE
    }

    fn step(&mut self, _: u128) {
        block_on(self.step_async())
    }
}

impl TownPopulationSim {
    pub fn new(params: TownPopulationSimParams, game_tx: &UpdateSender<Game>) -> TownPopulationSim {
        TownPopulationSim {
            params,
            game_tx: game_tx.clone_with_handle(HANDLE),
        }
    }

    async fn step_async(&mut self) {
        let route_summaries = self.get_route_summaries().await;
        let populations = self.compute_populations(route_summaries);
        self.set_target_populations(populations).await;
    }

    async fn get_route_summaries(&mut self) -> Vec<ControllerSummary> {
        let routes = self.get_routes_names().await;
        let mut out = vec![];
        for batch in routes.chunks(BATCH_SIZE) {
            out.append(
                &mut self
                    .get_controller_summaries_from_route_names(batch.to_vec())
                    .await,
            )
        }
        out
    }

    async fn get_routes_names(&mut self) -> Vec<String> {
        self.game_tx.update(|game| get_routes_names(game)).await
    }

    async fn get_controller_summaries_from_route_names(
        &self,
        route_names: Vec<String>,
    ) -> Vec<ControllerSummary> {
        self.game_tx
            .update(move |game| get_controller_summaries_from_route_names(game, route_names))
            .await
    }

    fn compute_populations(
        &self,
        route_summaries: Vec<ControllerSummary>,
    ) -> HashMap<V2<usize>, f64> {
        let mut out = HashMap::new();
        for summary in route_summaries {
            let activity = summary.get_activity();
            let activity_count = activity.len();
            for position in activity {
                *out.entry(position).or_insert(0.0) += (summary.traffic as f64
                    * self.params.population_per_traffic)
                    / activity_count as f64;
            }
        }
        out
    }

    async fn set_target_populations(&mut self, populations: HashMap<V2<usize>, f64>) {
        for town in self.get_towns().await {
            self.set_target_population(town, *populations.get(&town).unwrap_or(&0.0))
                .await
        }
    }

    async fn get_towns(&mut self) -> Vec<V2<usize>> {
        self.game_tx.update(move |game| get_towns(game)).await
    }

    async fn set_target_population(&mut self, town: V2<usize>, population: f64) {
        self.game_tx
            .update(move |game| set_target_population(game, town, population))
            .await
    }
}

fn get_routes_names(game: &mut Game) -> Vec<String> {
    game.game_state().routes.keys().cloned().collect()
}

fn get_controller_summaries_from_route_names(
    game: &mut Game,
    route_names: Vec<String>,
) -> Vec<ControllerSummary> {
    route_names
        .iter()
        .flat_map(|route_name| get_controller_summary_from_route_name(game, route_name))
        .collect()
}

fn get_controller_summary_from_route_name(
    game: &mut Game,
    route_name: &str,
) -> Option<ControllerSummary> {
    PositionSummary::from_route_name(game, route_name)
        .map(|summary| summary.to_controller_summary(&game.game_state().territory))
}

fn get_towns(game: &mut Game) -> Vec<V2<usize>> {
    game.game_state()
        .settlements
        .values()
        .filter(|settlement| is_town(settlement))
        .map(|settlement| settlement.position)
        .collect()
}

fn is_town(settlement: &Settlement) -> bool {
    if let SettlementClass::Town = settlement.class {
        true
    } else {
        false
    }
}

fn set_target_population(game: &mut Game, settlement: V2<usize>, target_population: f64) {
    let settlement = match game.game_state().settlements.get(&settlement) {
        Some(settlement) => settlement,
        None => return,
    };
    let updated_settlement = Settlement {
        target_population,
        ..*settlement
    };
    game.update_settlement(updated_settlement);
}

pub struct ControllerSummary {
    origin: Option<V2<usize>>,
    destination: Option<V2<usize>>,
    ports: HashSet<V2<usize>>,
    traffic: usize,
}

impl ControllerSummary {
    fn get_activity(&self) -> Vec<V2<usize>> {
        self.get_destination_activity()
            .into_iter()
            .chain(self.get_port_activity())
            .collect()
    }

    fn get_destination_activity(&self) -> Option<V2<usize>> {
        if self.destination == self.origin {
            None
        } else {
            self.destination
        }
    }

    fn get_port_activity<'a>(&'a self) -> impl Iterator<Item = V2<usize>> + 'a {
        self.ports
            .iter()
            .filter(move |&&port| Some(port) != self.origin)
            .filter(move |&&port| Some(port) != self.destination)
            .cloned()
    }
}

pub struct PositionSummary {
    origin: V2<usize>,
    destination: V2<usize>,
    ports: Vec<V2<usize>>,
    traffic: usize,
}

impl PositionSummary {
    fn from_route_name(game: &Game, route_name: &str) -> Option<PositionSummary> {
        game.game_state()
            .routes
            .get(route_name)
            .and_then(|route| PositionSummary::from_route(game, route))
    }

    fn from_route(game: &Game, route: &Route) -> Option<PositionSummary> {
        if let [origin, .., destination] = *route.path {
            Some(PositionSummary {
                origin,
                destination,
                ports: get_port_positions(game, &route.path).collect(),
                traffic: route.traffic,
            })
        } else {
            None
        }
    }

    fn to_controller_summary(&self, territory: &Territory) -> ControllerSummary {
        ControllerSummary {
            origin: get_controller(territory, &self.origin),
            destination: get_controller(territory, &self.destination),
            ports: self.get_port_controllers(territory),
            traffic: self.traffic,
        }
    }

    fn get_port_controllers(&self, territory: &Territory) -> HashSet<V2<usize>> {
        self.ports
            .iter()
            .flat_map(|position| get_controller(territory, position))
            .collect()
    }
}

fn get_controller(territory: &Territory, position: &V2<usize>) -> Option<V2<usize>> {
    territory
        .who_controls(position)
        .map(|claim| claim.controller)
}
