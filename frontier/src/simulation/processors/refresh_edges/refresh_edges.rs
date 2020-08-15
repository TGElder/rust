use super::*;

use crate::game::traits::{GetRoute, HasWorld, Micros};
use crate::pathfinder::traits::UpdateEdge;
use crate::travel_duration::TravelDuration;
use commons::edge::Edge;
use std::collections::HashSet;

const HANDLE: &str = "refresh_edges";
const BATCH_SIZE: usize = 128;

pub struct RefreshEdges<G, T, P>
where
    G: GetRoute + HasWorld + Micros,
    T: TravelDuration + 'static,
    P: UpdateEdge + Send + Sync + 'static,
{
    game: UpdateSender<G>,
    travel_duration: Arc<T>,
    pathfinder: Arc<RwLock<P>>,
}

impl<G, T, P> Processor for RefreshEdges<G, T, P>
where
    G: GetRoute + HasWorld + Micros,
    T: TravelDuration + 'static,
    P: UpdateEdge + Send + Sync + 'static,
{
    fn process(&mut self, state: State, instruction: &Instruction) -> State {
        let edges = match instruction {
            Instruction::RefreshEdges(edges) => edges.clone(),
            _ => return state,
        };
        self.refresh_edges_in_batches(state, edges)
    }
}

impl<G, T, P> RefreshEdges<G, T, P>
where
    G: GetRoute + HasWorld + Micros,
    T: TravelDuration + 'static,
    P: UpdateEdge + Send + Sync + 'static,
{
    pub fn new(
        game: &UpdateSender<G>,
        travel_duration: T,
        pathfinder: &Arc<RwLock<P>>,
    ) -> RefreshEdges<G, T, P> {
        RefreshEdges {
            game: game.clone_with_handle(HANDLE),
            travel_duration: Arc::new(travel_duration),
            pathfinder: pathfinder.clone(),
        }
    }

    fn refresh_edges_in_batches(&mut self, mut state: State, edges: HashSet<Edge>) -> State {
        let edges: Vec<Edge> = edges.into_iter().collect();
        for batch in edges.chunks(BATCH_SIZE) {
            state = self.refresh_edges(state, batch.to_vec());
        }
        state
    }

    fn refresh_edges(&mut self, state: State, edges: Vec<Edge>) -> State {
        let travel_duration = self.travel_duration.clone();
        let pathfinder = self.pathfinder.clone();
        block_on(async {
            self.game
                .update(move |game| refresh_edges(game, travel_duration, pathfinder, state, edges))
                .await
        })
    }
}

fn refresh_edges<G, T, P>(
    game: &mut G,
    travel_duration: Arc<T>,
    pathfinder: Arc<RwLock<P>>,
    mut state: State,
    edges: Vec<Edge>,
) -> State
where
    G: GetRoute + HasWorld + Micros,
    T: TravelDuration + 'static,
    P: UpdateEdge + 'static,
{
    for edge in edges {
        refresh_edge(
            game,
            travel_duration.as_ref(),
            &pathfinder,
            &mut state,
            edge,
        );
    }
    state
}

fn refresh_edge<G, T, P>(
    game: &mut G,
    travel_duration: &T,
    pathfinder: &Arc<RwLock<P>>,
    state: &mut State,
    edge: Edge,
) where
    G: GetRoute + HasWorld + Micros,
    T: TravelDuration + 'static,
    P: UpdateEdge + 'static,
{
    let edge_traffic = get_edge_traffic(game, travel_duration, &state, &edge);
    if let Some(instruction) = try_build_road(game, pathfinder, &edge_traffic) {
        state.build_queue.push(instruction);
    }
}
