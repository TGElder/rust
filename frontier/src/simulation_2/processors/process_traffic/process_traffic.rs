use super::*;

use crate::game::traits::{HasWorld, Micros, Nations, Routes, Settlements, WhoControlsTile};
use crate::route::{RouteSet, RouteSetKey};
use crate::travel_duration::TravelDuration;

const HANDLE: &str = "update_traffic";

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
    fn process(&mut self, state: State, instruction: &Instruction) -> State {
        let (key, route_set) = match instruction {
            Instruction::GetRouteChanges { key, route_set } => (key, route_set),
            _ => return state,
        };
        self.process_traffic(state, *key, route_set.clone())
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

    fn process_traffic(&mut self, state: State, key: RouteSetKey, route_set: RouteSet) -> State {
        let travel_duration = self.travel_duration.clone();
        block_on(async {
            self.game
                .update(move |game| process_traffic(game, travel_duration, state, key, route_set))
                .await
        })
    }
}

fn process_traffic<G, T>(
    game: &mut G,
    travel_duration: Arc<T>,
    mut state: State,
    key: RouteSetKey,
    route_set: RouteSet,
) -> State
where
    G: HasWorld + Micros + Nations + Routes + Settlements + WhoControlsTile,
    T: TravelDuration + 'static,
{
    let route_changes = update_routes_and_get_changes(game, &key, &route_set);
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
