use super::*;

use crate::game::traits::{BuildRoad, GetRoute, HasWorld};
use crate::pathfinder::traits::UpdateEdge;
use crate::travel_duration::TravelDuration;
use commons::edge::Edge;
use commons::futures::FutureExt;
use std::collections::HashSet;

const NAME: &str = "refresh_edges";
const BATCH_SIZE: usize = 128;

pub struct RefreshEdges<G, R, T, P>
where
    G: GetRoute + HasWorld + Send,
    R: BuildRoad + Clone + Send + Sync + 'static,
    T: TravelDuration + 'static,
    P: UpdateEdge + Send + Sync + 'static,
{
    game: FnSender<G>,
    build_road: R,
    travel_duration: Arc<T>,
    pathfinder: Arc<RwLock<P>>,
}

#[async_trait]
impl<G, R, T, P> Processor for RefreshEdges<G, R, T, P>
where
    G: GetRoute + HasWorld + Send,
    R: BuildRoad + Clone + Send + Sync + 'static,
    T: TravelDuration + 'static,
    P: UpdateEdge + Send + Sync + 'static,
{
    async fn process(&mut self, state: State, instruction: &Instruction) -> State {
        let edges = match instruction {
            Instruction::RefreshEdges(edges) => edges.clone(),
            _ => return state,
        };
        self.refresh_edges_in_batches(state, edges).await
    }
}

impl<G, R, T, P> RefreshEdges<G, R, T, P>
where
    G: GetRoute + HasWorld + Send,
    R: BuildRoad + Clone + Send + Sync + 'static,
    T: TravelDuration + 'static,
    P: UpdateEdge + Send + Sync + 'static,
{
    pub fn new(
        game: &FnSender<G>,
        build_road: R,
        travel_duration: T,
        pathfinder: &Arc<RwLock<P>>,
    ) -> RefreshEdges<G, R, T, P> {
        RefreshEdges {
            game: game.clone_with_name(NAME),
            build_road,
            travel_duration: Arc::new(travel_duration),
            pathfinder: pathfinder.clone(),
        }
    }

    async fn refresh_edges_in_batches(&mut self, mut state: State, edges: HashSet<Edge>) -> State {
        let edges: Vec<Edge> = edges.into_iter().collect();
        for batch in edges.chunks(BATCH_SIZE) {
            state = self.refresh_edges(state, batch.to_vec()).await;
        }
        state
    }

    async fn refresh_edges(&mut self, state: State, edges: Vec<Edge>) -> State {
        let build_road = self.build_road.clone();
        let travel_duration = self.travel_duration.clone();
        let pathfinder = self.pathfinder.clone();
        self.game
            .send_future(move |game| {
                refresh_edges(game, build_road, travel_duration, pathfinder, state, edges).boxed()
            })
            .await
    }
}

async fn refresh_edges<G, R, T, P>(
    game: &mut G,
    mut build_road: R,
    travel_duration: Arc<T>,
    pathfinder: Arc<RwLock<P>>,
    mut state: State,
    edges: Vec<Edge>,
) -> State
where
    G: GetRoute + HasWorld + Send,
    R: BuildRoad + Clone + Send + Sync + 'static,
    T: TravelDuration + 'static,
    P: UpdateEdge + Send + Sync + 'static,
{
    for edge in edges {
        refresh_edge(
            game,
            &mut build_road,
            travel_duration.as_ref(),
            &pathfinder,
            &mut state,
            edge,
        )
        .await;
    }
    state
}

async fn refresh_edge<G, R, T, P>(
    game: &mut G,
    build_road: &mut R,
    travel_duration: &T,
    pathfinder: &Arc<RwLock<P>>,
    state: &mut State,
    edge: Edge,
) where
    G: GetRoute + HasWorld + Send,
    R: BuildRoad + Clone + Send + Sync + 'static,
    T: TravelDuration + 'static,
    P: UpdateEdge + Send + Sync + 'static,
{
    let edge_traffic = get_edge_traffic(game, travel_duration, &state, &edge);
    if let Some(instruction) = try_build_road(game, pathfinder, &edge_traffic) {
        state.build_queue.insert(instruction);
    }
    try_remove_road(state, game, build_road, pathfinder, &edge_traffic).await;
}
