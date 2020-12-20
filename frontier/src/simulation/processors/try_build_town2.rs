use std::collections::{HashMap};
use std::time::Duration;

use commons::grid::Grid;
use commons::log::info;

use crate::game::traits::GetRoute;
use crate::route::{RouteKey, RouteSet, RouteSetKey};
use crate::settlement::{Settlement, SettlementClass};
use crate::traits::{GetSettlement, RandomTownName, SendGame, SendWorld, WhoControlsTile};
use crate::world::World;

use super::*;
pub struct TryBuildTown<X> {
    x: X,
}

#[async_trait]
impl<X> Processor for TryBuildTown<X>
where
    X: GetSettlement + RandomTownName + WhoControlsTile + SendGame + SendWorld + Send + Sync + 'static,
{
    async fn process(&mut self, mut state: State, instruction: &Instruction) -> State {
        let positions = match instruction {
            Instruction::RefreshPositions(positions) => positions.clone(),
            _ => return state,
        };

        info!("Checking {} positions", positions.len());

        let candidates: Vec<Candidate> = positions
            .into_iter()
            .flat_map(|position| to_candidate(&state, position))
            .collect();
        if candidates.is_empty() {
            return state;
        }

        info!("{} positions remain after checking traffic", candidates.len());

        let candidates = self.not_controlled(candidates).await;
        if candidates.is_empty() {
            return state;
        }

        info!("{} positions remain after checking control", candidates.len());

        let candidates = self.x.send_world(move |world| tile_candidates(world, candidates)).await;
        if candidates.is_empty() {
            return state;
        }

        info!("{} tile candidates", candidates.len());

        let final_candidates = self.x.send_game(move |game| final_candidates(&game.game_state().routes, candidates)).await;
        for FinalCandidate{position, first_visit, first_visit_settlement} in final_candidates {
            let nation = unwrap_or!(self.x.get_settlement(first_visit_settlement).await, continue).nation;
            let name = self.x.random_town_name(nation.clone()).await.unwrap();
            let settlement = Settlement{
                class: SettlementClass::Town,
                position,
                name,
                nation,
                current_population: state.params.initial_town_population,
                target_population: state.params.initial_town_population,
                gap_half_life: Duration::from_millis(0),
                last_population_update_micros: first_visit,
            };

            state.build_queue.insert(BuildInstruction{
                what: Build::Town(settlement),
                when: first_visit
            });

        }

        info!("Done");

        state
    }
}

impl<X> TryBuildTown<X>
where
    X: WhoControlsTile,
{
    pub fn new(x: X) -> TryBuildTown<X> {
        TryBuildTown{
            x
        }
    }

    async fn not_controlled(&self, candidates: Vec<Candidate>) -> Vec<Candidate>
    where
        X: WhoControlsTile,
    {
        let mut out = Vec::with_capacity(candidates.len());
        for candidate in candidates.into_iter() {
            if self
                .x
                .who_controls_tile(&candidate.position)
                .await
                .is_none()
            {
                out.push(candidate);
            }
        }
        out
    }
}

struct Candidate {
    position: V2<usize>,
    routes: Vec<RouteKey>,
}

impl Candidate{

    fn to_final_candidate(self, routes: &HashMap<RouteSetKey, RouteSet>) -> FinalCandidate {
        let (route_key, route) = self.routes.into_iter()
            .flat_map(|route_key| routes.get_route(&route_key).map(|routes| (route_key, routes)))
            .min_by_key(|(_, route)| route.start_micros + route.duration.as_micros())
            .unwrap();
        FinalCandidate{
            position: self.position,
            first_visit: route.start_micros + route.duration.as_micros(),
            first_visit_settlement: route_key.settlement,
            
        }
    }
}

struct FinalCandidate {
    position: V2<usize>,
    first_visit: u128,
    first_visit_settlement: V2<usize>,
}

fn to_candidate(state: &State, position: V2<usize>) -> Option<Candidate> {
    let traffic = ok_or!(state.traffic.get(&position), return None);
    if traffic.is_empty() {
        return None;
    }
    let routes: Vec<RouteKey> = traffic
        .iter()
        .filter(|route| {
            route.destination == position
                || state
                    .route_to_ports
                    .get(route)
                    .map_or(false, |ports| ports.contains(&position))
        })
        .cloned()
        .collect();

    if routes.is_empty() {
        return None;
    }
    Some(Candidate { position, routes })
}

fn tile_candidates(world: &World, candidates: Vec<Candidate>) -> Vec<Candidate>
{
    candidates.iter()
        .flat_map(|Candidate{position, routes}| world.get_adjacent_tiles_in_bounds(position).into_iter().map(move |tile| Candidate{position: tile, routes: routes.clone()}))
        .filter(|Candidate{position, ..}| world.get_cell(position).map_or(false, |cell| cell.visible))
        .filter(|Candidate{position, ..}| !world.is_sea(position))
        .collect()
}

fn final_candidates(routes: &HashMap<RouteSetKey, RouteSet>, candidates: Vec<Candidate>) -> Vec<FinalCandidate>
{
    candidates.into_iter().map(|candidate| candidate.to_final_candidate(routes)).collect()
}