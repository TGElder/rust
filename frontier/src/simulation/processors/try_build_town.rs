use std::collections::HashMap;
use std::time::Duration;

use commons::grid::Grid;
use commons::log::trace;

use crate::game::traits::GetRoute;
use crate::route::{Route, RouteKey};
use crate::settlement::{Settlement, SettlementClass};
use crate::traits::{GetSettlement, RandomTownName, SendGame, SendWorld, WhoControlsTile};

use super::*;
pub struct TryBuildTown<X> {
    x: X,
}

#[async_trait]
impl<X> Processor for TryBuildTown<X>
where
    X: GetSettlement
        + RandomTownName
        + SendGame
        + SendWorld
        + WhoControlsTile
        + Send
        + Sync
        + 'static,
{
    async fn process(&mut self, mut state: State, instruction: &Instruction) -> State {
        let positions = match instruction {
            Instruction::RefreshPositions(positions) => positions.clone(),
            _ => return state,
        };

        let start = std::time::Instant::now();
        let mut count: usize = 0;
        let position_count = positions.len();
        for position in positions {
            if self.process_position(&mut state, position).await {
                count += 1;
            }
        }

        trace!(
            "Sent {}/{} build instructions in {}ms",
            count,
            position_count,
            start.elapsed().as_millis()
        );

        state
    }
}

impl<X> TryBuildTown<X>
where
    X: GetSettlement + RandomTownName + SendGame + SendWorld + WhoControlsTile,
{
    pub fn new(x: X) -> TryBuildTown<X> {
        TryBuildTown { x }
    }

    async fn process_position(&mut self, state: &mut State, position: V2<usize>) -> bool {
        let traffic = ok_or!(state.traffic.get(&position), return false);
        if traffic.is_empty() {
            return false;
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
            return false;
        }

        if self.x.who_controls_tile(&position).await.is_some() {
            return false;
        }

        let tiles: Vec<V2<usize>> = self
            .x
            .send_world(move |world| {
                world
                    .get_adjacent_tiles_in_bounds(&position)
                    .into_iter()
                    .filter(|tile| {
                        world.get_cell(tile).map_or(false, |cell| cell.visible)
                            && !world.is_sea(tile)
                    })
                    .collect()
            })
            .await;

        if tiles.is_empty() {
            return false;
        }

        let routes: HashMap<RouteKey, Route> = self
            .x
            .send_game(move |game| {
                routes
                    .into_iter()
                    .flat_map(|route_key| {
                        game.game_state()
                            .routes
                            .get_route(&route_key)
                            .map(|route| (route_key, route.clone()))
                    })
                    .collect()
            })
            .await;

        let (first_route_key, first_route) = routes
            .into_iter()
            .min_by_key(|(_, route)| route.start_micros + route.duration.as_micros())
            .unwrap();
        let first_visit = first_route.start_micros + first_route.duration.as_micros(); // TODO recalced

        let nation = unwrap_or!(
            self.x.get_settlement(first_route_key.settlement).await,
            return false
        )
        .nation;
        let name = ok_or!(self.x.random_town_name(nation.clone()).await, return false);

        let settlement = Settlement {
            class: SettlementClass::Town,
            position,
            name,
            nation,
            current_population: state.params.initial_town_population,
            target_population: state.params.initial_town_population,
            gap_half_life: Duration::from_millis(0),
            last_population_update_micros: first_visit,
        };

        state.build_queue.insert(BuildInstruction {
            what: Build::Town(settlement),
            when: first_visit,
        });

        true
    }
}
