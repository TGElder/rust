use super::*;

use crate::game::traits::{HasWorld, Micros, Nations, Routes, Settlements, WhoControlsTile};
use crate::route::{RouteSet, RouteSetKey};
use crate::travel_duration::TravelDuration;

const HANDLE: &str = "update_traffic";
const BATCH_SIZE: usize = 16;

pub struct ProcessTraffic<G, T>
where
    G: HasWorld + Micros + Nations + Routes + Settlements + WhoControlsTile,
    T: TravelDuration + 'static,
{
    game: UpdateSender<G>,
    travel_duration: Arc<T>,
}

impl<G, T> Processor for ProcessTraffic<G, T>
where
    G: HasWorld + Micros + Nations + Routes + Settlements + WhoControlsTile,
    T: TravelDuration + 'static,
{
    fn process(&mut self, mut state: State, instruction: &Instruction) -> State {
        let (key, route_set) = match instruction {
            Instruction::GetRouteChanges { key, route_set } => (key, route_set),
            _ => return state,
        };
        let route_changes = self.update_routes_and_get_changes(*key, route_set.clone());
        for batch in route_changes.chunks(BATCH_SIZE) {
            state = self.process_route_changes(state, batch.to_vec());
        }
        state
    }
}

impl<G, T> ProcessTraffic<G, T>
where
    G: HasWorld + Micros + Nations + Routes + Settlements + WhoControlsTile,
    T: TravelDuration + 'static,
{
    pub fn new(game: &UpdateSender<G>, travel_duration: T) -> ProcessTraffic<G, T> {
        ProcessTraffic {
            game: game.clone_with_handle(HANDLE),
            travel_duration: Arc::new(travel_duration),
        }
    }

    fn update_routes_and_get_changes(
        &mut self,
        key: RouteSetKey,
        route_set: RouteSet,
    ) -> Vec<RouteChange> {
        block_on(async {
            self.game
                .update(move |game| update_routes_and_get_changes(game, &key, &route_set))
                .await
        })
    }

    fn process_route_changes(&mut self, state: State, route_changes: Vec<RouteChange>) -> State {
        let travel_duration = self.travel_duration.clone();
        block_on(async {
            self.game
                .update(move |game| {
                    process_route_changes(game, travel_duration, state, route_changes)
                })
                .await
        })
    }
}

fn process_route_changes<G, T>(
    game: &mut G,
    travel_duration: Arc<T>,
    mut state: State,
    route_changes: Vec<RouteChange>,
) -> State
where
    G: HasWorld + Micros + Nations + Routes + Settlements + WhoControlsTile,
    T: TravelDuration + 'static,
{
    for route_change in route_changes {
        for position in update_traffic_and_get_changes(&mut state, &route_change) {
            let traffic = get_traffic(game, &mut state, &position);
            if let Some(instruction) = try_build_destination_town(game, &traffic) {
                state.build_queue.push(instruction);
            }
        }
        for edge in update_edge_traffic_and_get_changes(&mut state, &route_change) {
            let edge_traffic = get_edge_traffic(game, travel_duration.as_ref(), &state, &edge);
            if let Some(instruction) = try_build_road(&edge_traffic) {
                state.build_queue.push(instruction);
            }
        }
    }
    state
}
