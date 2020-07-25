use super::*;

use crate::build_service::{BuildInstruction, BuildQueue};
use crate::game::traits::{HasWorld, Micros, Nations, Routes, Settlements, WhoControlsTile};
use crate::route::{RouteSet, RouteSetKey};
use crate::travel_duration::TravelDuration;

const GAME_HANDLE: &str = "update_traffic_to_game";
const BUILDER_HANDLE: &str = "update_traffic_to_builder";

pub struct ProcessTraffic<G, T, B>
where
    G: HasWorld + Micros + Nations + Routes + Settlements + WhoControlsTile,
    T: TravelDuration + 'static,
    B: BuildQueue + 'static,
{
    game: UpdateSender<G>,
    travel_duration: Arc<T>,
    builder: UpdateSender<B>,
}

impl<G, T, B> Processor for ProcessTraffic<G, T, B>
where
    G: HasWorld + Micros + Nations + Routes + Settlements + WhoControlsTile,
    T: TravelDuration + 'static,
    B: BuildQueue + 'static,
{
    fn process(&mut self, state: State, instruction: &Instruction) -> State {
        let (key, route_set) = match instruction {
            Instruction::GetRouteChanges { key, route_set } => (key, route_set),
            _ => return state,
        };
        self.process_traffic(state, *key, route_set.clone())
    }
}

impl<G, T, B> ProcessTraffic<G, T, B>
where
    G: HasWorld + Micros + Nations + Routes + Settlements + WhoControlsTile,
    T: TravelDuration + 'static,
    B: BuildQueue + 'static,
{
    pub fn new(
        game: &UpdateSender<G>,
        travel_duration: T,
        builder: &UpdateSender<B>,
    ) -> ProcessTraffic<G, T, B> {
        ProcessTraffic {
            game: game.clone_with_handle(GAME_HANDLE),
            travel_duration: Arc::new(travel_duration),
            builder: builder.clone_with_handle(BUILDER_HANDLE),
        }
    }

    fn process_traffic(&mut self, state: State, key: RouteSetKey, route_set: RouteSet) -> State {
        let travel_duration = self.travel_duration.clone();
        let builder = self.builder.clone();
        block_on(async {
            self.game
                .update(move |game| {
                    process_traffic(game, travel_duration, builder, state, key, route_set)
                })
                .await
        })
    }
}

fn process_traffic<G, T, B>(
    game: &mut G,
    travel_duration: Arc<T>,
    mut builder: UpdateSender<B>,
    mut state: State,
    key: RouteSetKey,
    route_set: RouteSet,
) -> State
where
    G: HasWorld + Micros + Nations + Routes + Settlements + WhoControlsTile,
    T: TravelDuration + 'static,
    B: BuildQueue + 'static,
{
    let route_changes = update_routes_and_get_changes(game, &key, &route_set);
    for route_change in route_changes {
        for position in update_traffic_and_get_changes(&mut state, &route_change) {
            let traffic = get_traffic(game, &mut state, &position);
            if let Some(instruction) = try_build_destination_town(game, &traffic) {
                build(&mut builder, instruction);
            }
        }
        for edge in update_edge_traffic_and_get_changes(&mut state, &route_change) {
            let edge_traffic = get_edge_traffic(game, travel_duration.as_ref(), &state, &edge);
            if let Some(instruction) = try_build_road(&edge_traffic) {
                build(&mut builder, instruction);
            }
        }
    }
    state
}

fn build<B>(builder: &mut UpdateSender<B>, instruction: BuildInstruction)
where
    B: BuildQueue,
{
    builder.update(|builder| builder.queue(instruction));
}
