use super::*;

use crate::avatar::CheckForPort;
use crate::game::traits::{HasWorld, Micros, Nations, Routes, Settlements, WhoControlsTile};
use crate::pathfinder::traits::UpdateEdge;
use crate::route::{RouteSet, RouteSetKey};
use crate::travel_duration::TravelDuration;
use commons::edge::Edge;
use std::collections::HashSet;

const HANDLE: &str = "update_traffic";
const BATCH_SIZE: usize = 128;

pub struct ProcessTraffic<G, T, P>
where
    G: CheckForPort + HasWorld + Micros + Nations + Routes + Settlements + WhoControlsTile,
    T: TravelDuration + 'static,
    P: UpdateEdge + Send + Sync + 'static,
{
    game: UpdateSender<G>,
    travel_duration: Arc<T>,
    pathfinder: Arc<RwLock<P>>,
}

impl<G, T, P> Processor for ProcessTraffic<G, T, P>
where
    G: CheckForPort + HasWorld + Micros + Nations + Routes + Settlements + WhoControlsTile,
    T: TravelDuration + 'static,
    P: UpdateEdge + Send + Sync + 'static,
{
    fn process(&mut self, mut state: State, instruction: &Instruction) -> State {
        let (key, route_set) = match instruction {
            Instruction::GetRouteChanges { key, route_set } => (key, route_set),
            _ => return state,
        };
        let route_changes = self.update_routes_and_get_changes(*key, route_set.clone());
        state = self.update_position_traffic_and_process_position_changes(state, &route_changes);
        state = self.update_edge_traffic_and_process_edge_changes(state, &route_changes);
        state
    }
}

impl<G, T, P> ProcessTraffic<G, T, P>
where
    G: CheckForPort + HasWorld + Micros + Nations + Routes + Settlements + WhoControlsTile,
    T: TravelDuration + 'static,
    P: UpdateEdge + Send + Sync + 'static,
{
    pub fn new(
        game: &UpdateSender<G>,
        travel_duration: T,
        pathfinder: &Arc<RwLock<P>>,
    ) -> ProcessTraffic<G, T, P> {
        ProcessTraffic {
            game: game.clone_with_handle(HANDLE),
            travel_duration: Arc::new(travel_duration),
            pathfinder: pathfinder.clone(),
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

    fn update_position_traffic_and_process_position_changes(
        &mut self,
        mut state: State,
        route_changes: &[RouteChange],
    ) -> State {
        let position_changes: HashSet<V2<usize>> = route_changes
            .iter()
            .flat_map(|route_change| update_traffic_and_get_changes(&mut state, route_change))
            .collect();
        let position_changes: Vec<V2<usize>> = position_changes.into_iter().collect();
        for batch in position_changes.chunks(BATCH_SIZE) {
            state = self.process_traffic_position_changes(state, batch.to_vec());
        }
        state
    }

    fn process_traffic_position_changes(
        &mut self,
        state: State,
        traffic_changes: Vec<V2<usize>>,
    ) -> State {
        block_on(async {
            self.game
                .update(move |game| process_traffic_position_changes(game, state, traffic_changes))
                .await
        })
    }

    fn update_edge_traffic_and_process_edge_changes(
        &mut self,
        mut state: State,
        route_changes: &[RouteChange],
    ) -> State {
        let edge_changes: HashSet<Edge> = route_changes
            .iter()
            .flat_map(|route_change| update_edge_traffic_and_get_changes(&mut state, route_change))
            .collect();
        let edge_changes: Vec<Edge> = edge_changes.into_iter().collect();
        for batch in edge_changes.chunks(BATCH_SIZE) {
            state = self.process_traffic_edge_changes(state, batch.to_vec());
        }
        state
    }

    fn process_traffic_edge_changes(&mut self, state: State, traffic_changes: Vec<Edge>) -> State {
        let travel_duration = self.travel_duration.clone();
        let pathfinder = self.pathfinder.clone();
        block_on(async {
            self.game
                .update(move |game| {
                    process_traffic_edge_changes(
                        game,
                        travel_duration,
                        pathfinder,
                        state,
                        traffic_changes,
                    )
                })
                .await
        })
    }
}

fn process_traffic_position_changes<G>(
    game: &mut G,
    mut state: State,
    traffic_changes: Vec<V2<usize>>,
) -> State
where
    G: CheckForPort
        + CheckForPort
        + HasWorld
        + Micros
        + Nations
        + Routes
        + Settlements
        + WhoControlsTile,
{
    for position in traffic_changes {
        let traffic = get_traffic(game, &state, &position);
        if let Some(instruction) = try_build_destination_town(game, &traffic) {
            state.build_queue.push(instruction);
        }
        if let Some(instruction) = try_build_crops(game, &traffic) {
            state.build_queue.push(instruction);
        }
    }
    state
}

fn process_traffic_edge_changes<G, T, P>(
    game: &mut G,
    travel_duration: Arc<T>,
    pathfinder: Arc<RwLock<P>>,
    mut state: State,
    traffic_changes: Vec<Edge>,
) -> State
where
    G: CheckForPort + HasWorld + Micros + Routes,
    T: TravelDuration + 'static,
    P: UpdateEdge + 'static,
{
    for edge in traffic_changes {
        let edge_traffic = get_edge_traffic(game, travel_duration.as_ref(), &state, &edge);
        if let Some(instruction) = try_build_road(game, &pathfinder, &edge_traffic) {
            state.build_queue.push(instruction);
        }
    }
    state
}
