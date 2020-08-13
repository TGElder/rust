use super::*;

use crate::game::traits::{GetRoute, HasWorld, Micros};
use crate::pathfinder::traits::UpdateEdge;
use crate::travel_duration::TravelDuration;
use commons::edge::Edge;
use std::collections::HashSet;

const HANDLE: &str = "process_edge_traffic";
const BATCH_SIZE: usize = 128;

pub struct ProcessEdgeTraffic<G, T, P>
where
    G: GetRoute + HasWorld + Micros,
    T: TravelDuration + 'static,
    P: UpdateEdge + Send + Sync + 'static,
{
    game: UpdateSender<G>,
    travel_duration: Arc<T>,
    pathfinder: Arc<RwLock<P>>,
}

impl<G, T, P> Processor for ProcessEdgeTraffic<G, T, P>
where
    G: GetRoute + HasWorld + Micros,
    T: TravelDuration + 'static,
    P: UpdateEdge + Send + Sync + 'static,
{
    fn process(&mut self, mut state: State, instruction: &Instruction) -> State {
        let route_changes = match instruction {
            Instruction::ProcessRouteChanges(route_changes) => route_changes.clone(),
            _ => return state,
        };
        let edge_changes = update_all_edge_traffic_and_get_changes(&mut state, &route_changes);
        self.process_edge_changes_in_batches(state, edge_changes)
    }
}

impl<G, T, P> ProcessEdgeTraffic<G, T, P>
where
    G: GetRoute + HasWorld + Micros,
    T: TravelDuration + 'static,
    P: UpdateEdge + Send + Sync + 'static,
{
    pub fn new(
        game: &UpdateSender<G>,
        travel_duration: T,
        pathfinder: &Arc<RwLock<P>>,
    ) -> ProcessEdgeTraffic<G, T, P> {
        ProcessEdgeTraffic {
            game: game.clone_with_handle(HANDLE),
            travel_duration: Arc::new(travel_duration),
            pathfinder: pathfinder.clone(),
        }
    }

    fn process_edge_changes_in_batches(
        &mut self,
        mut state: State,
        edge_changes: HashSet<Edge>,
    ) -> State {
        let edge_changes: Vec<Edge> = edge_changes.into_iter().collect();
        for batch in edge_changes.chunks(BATCH_SIZE) {
            state = self.process_edge_changes(state, batch.to_vec());
        }
        state
    }

    fn process_edge_changes(&mut self, state: State, edge_changes: Vec<Edge>) -> State {
        let travel_duration = self.travel_duration.clone();
        let pathfinder = self.pathfinder.clone();
        block_on(async {
            self.game
                .update(move |game| {
                    process_edge_changes(game, travel_duration, pathfinder, state, edge_changes)
                })
                .await
        })
    }
}

fn process_edge_changes<G, T, P>(
    game: &mut G,
    travel_duration: Arc<T>,
    pathfinder: Arc<RwLock<P>>,
    mut state: State,
    edge_changes: Vec<Edge>,
) -> State
where
    G: GetRoute + HasWorld + Micros,
    T: TravelDuration + 'static,
    P: UpdateEdge + 'static,
{
    for edge_change in edge_changes {
        process_edge_change(
            game,
            travel_duration.as_ref(),
            &pathfinder,
            &mut state,
            edge_change,
        );
    }
    state
}

fn process_edge_change<G, T, P>(
    game: &mut G,
    travel_duration: &T,
    pathfinder: &Arc<RwLock<P>>,
    state: &mut State,
    edge_change: Edge,
) where
    G: GetRoute + HasWorld + Micros,
    T: TravelDuration + 'static,
    P: UpdateEdge + 'static,
{
    let edge_traffic = get_edge_traffic(game, travel_duration, &state, &edge_change);
    if let Some(instruction) = try_build_road(game, pathfinder, &edge_traffic) {
        state.build_queue.push(instruction);
    }
}
