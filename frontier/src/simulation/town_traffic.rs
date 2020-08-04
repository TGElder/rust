use super::*;

use crate::route::*;
use crate::settlement::*;
use crate::territory::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::convert::TryInto;
use std::default::Default;

const HANDLE: &str = "town_traffic_sim";
const BATCH_SIZE: usize = 128;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct TownTrafficSimParams {
    population_per_traffic: f64,
    nation_switch_pc_threshold: f32,
}

impl Default for TownTrafficSimParams {
    fn default() -> TownTrafficSimParams {
        TownTrafficSimParams {
            population_per_traffic: 0.5,
            nation_switch_pc_threshold: 0.5,
        }
    }
}

pub struct TownTrafficSim {
    params: TownTrafficSimParams,
    game_tx: UpdateSender<Game>,
}

impl Step for TownTrafficSim {
    fn name(&self) -> &'static str {
        HANDLE
    }

    fn init(&mut self) {}

    fn step(&mut self, _: u128) {
        block_on(self.step_async())
    }
}

impl TownTrafficSim {
    pub fn new(params: TownTrafficSimParams, game_tx: &UpdateSender<Game>) -> TownTrafficSim {
        TownTrafficSim {
            params,
            game_tx: game_tx.clone_with_handle(HANDLE),
        }
    }

    async fn step_async(&mut self) {
        let route_summaries = self.get_route_summaries().await;
        let updates = self.compute_updates(route_summaries);
        self.update_settlements(updates).await;
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

    fn compute_updates(
        &self,
        route_summaries: Vec<ControllerSummary>,
    ) -> HashMap<V2<usize>, SettlementUpdate> {
        let mut out = HashMap::new();
        let mut position_to_traffic = HashMap::new();
        for summary in route_summaries {
            let activity = summary.get_activity();
            let activity_count = activity.len();
            for position in activity {
                let update = out.entry(position).or_insert_with(SettlementUpdate::new);
                update.population += (summary.traffic as f64 * self.params.population_per_traffic)
                    / activity_count as f64;
                update.total_duration += summary.duration * summary.traffic.try_into().unwrap();
                update.traffic += summary.traffic;

                let nation_to_traffic = position_to_traffic
                    .entry(position)
                    .or_insert_with(HashMap::new);
                *nation_to_traffic.entry(summary.nation.clone()).or_insert(0) += summary.traffic;
            }
        }

        self.set_parent_updates(position_to_traffic, &mut out);

        out
    }

    fn set_parent_updates(
        &self,
        mut position_to_traffic: HashMap<V2<usize>, HashMap<String, usize>>,
        updates: &mut HashMap<V2<usize>, SettlementUpdate>,
    ) {
        for (position, traffic) in position_to_traffic.drain() {
            let update = updates
                .entry(position)
                .or_insert_with(SettlementUpdate::new);
            self.set_parent_update(traffic, update);
        }
    }

    fn set_parent_update(
        &self,
        mut nation_to_traffic: HashMap<String, usize>,
        update: &mut SettlementUpdate,
    ) {
        let total: usize = nation_to_traffic.values().sum();
        let (nation, max) = nation_to_traffic
            .drain()
            .max_by(|a, b| a.1.cmp(&b.1))
            .unwrap();
        if max as f32 / total as f32 >= self.params.nation_switch_pc_threshold {
            update.nation = Some(nation);
        }
    }

    async fn update_settlements(&mut self, mut updates: HashMap<V2<usize>, SettlementUpdate>) {
        for town in self.get_towns().await {
            self.update_settlement(
                town,
                updates.remove(&town).unwrap_or_else(SettlementUpdate::new),
            )
            .await
        }
    }

    async fn get_towns(&mut self) -> Vec<V2<usize>> {
        self.game_tx.update(move |game| get_towns(game)).await
    }

    async fn update_settlement(&mut self, town: V2<usize>, update: SettlementUpdate) {
        self.game_tx
            .update(move |game| update_settlement(game, town, update))
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

fn update_settlement(game: &mut Game, settlement: V2<usize>, update: SettlementUpdate) {
    let settlement = unwrap_or!(game.game_state().settlements.get(&settlement), return);
    let updated_settlement = Settlement {
        target_population: update.population,
        gap_half_life: update.avg_duration().map(|duration| duration * 2),
        name: settlement.name.clone(),
        nation: update.nation.unwrap_or_else(|| settlement.nation.clone()),
        ..*settlement
    };
    game.update_settlement(updated_settlement);
}
#[derive(Debug)]
pub struct ControllerSummary {
    origin: V2<usize>,
    nation: String,
    destination: Option<V2<usize>>,
    ports: HashSet<V2<usize>>,
    traffic: usize,
    duration: Duration,
}

impl ControllerSummary {
    fn get_activity(&self) -> Vec<V2<usize>> {
        if self.destination.is_none() {
            return vec![];
        }
        self.get_destination_activity()
            .into_iter()
            .chain(self.get_port_activity())
            .collect()
    }

    fn get_destination_activity(&self) -> Option<V2<usize>> {
        if self.destination == Some(self.origin) {
            None
        } else {
            self.destination
        }
    }

    fn get_port_activity<'a>(&'a self) -> impl Iterator<Item = V2<usize>> + 'a {
        self.ports
            .iter()
            .filter(move |&&port| port != self.origin)
            .filter(move |&&port| Some(port) != self.destination)
            .cloned()
    }
}

pub struct PositionSummary {
    origin: V2<usize>,
    nation: String,
    destination: V2<usize>,
    ports: Vec<V2<usize>>,
    traffic: usize,
    duration: Duration,
}

impl PositionSummary {
    fn from_route_name(game: &Game, route_name: &str) -> Option<PositionSummary> {
        game.game_state()
            .routes
            .get(route_name)
            .and_then(|route| PositionSummary::from_route(game, route))
    }

    fn from_route(game: &Game, route: &Route) -> Option<PositionSummary> {
        let settlement = game.game_state().settlements.get(&route.settlement)?;
        if let Some(&destination) = route.path.last() {
            Some(PositionSummary {
                origin: route.settlement,
                nation: settlement.nation.clone(),
                destination,
                ports: get_port_positions(game, &route.path).collect(),
                traffic: route.traffic,
                duration: route.duration + get_extra_duration(game, settlement),
            })
        } else {
            None
        }
    }

    fn to_controller_summary(&self, territory: &Territory) -> ControllerSummary {
        ControllerSummary {
            origin: self.origin,
            nation: self.nation.clone(),
            destination: get_controller(territory, &self.destination),
            ports: self.get_port_controllers(territory),
            traffic: self.traffic,
            duration: self.duration,
        }
    }

    fn get_port_controllers(&self, territory: &Territory) -> HashSet<V2<usize>> {
        self.ports
            .iter()
            .flat_map(|position| get_controller(territory, position))
            .collect()
    }
}

fn get_extra_duration(game: &Game, settlement: &Settlement) -> Duration {
    if settlement.class == SettlementClass::Homeland {
        game.game_state().params.homeland_distance
    } else {
        Duration::from_secs(0)
    }
}

fn get_controller(territory: &Territory, position: &V2<usize>) -> Option<V2<usize>> {
    territory
        .who_controls(position)
        .map(|claim| claim.controller)
}

#[derive(Clone)]
pub struct SettlementUpdate {
    population: f64,
    total_duration: Duration,
    traffic: usize,
    nation: Option<String>,
}

impl SettlementUpdate {
    fn new() -> SettlementUpdate {
        SettlementUpdate {
            population: 0.0,
            total_duration: Duration::from_secs(0),
            traffic: 0,
            nation: None,
        }
    }

    fn avg_duration(&self) -> Option<Duration> {
        if self.traffic == 0 {
            None
        } else {
            Some(self.total_duration / self.traffic.try_into().unwrap())
        }
    }
}
