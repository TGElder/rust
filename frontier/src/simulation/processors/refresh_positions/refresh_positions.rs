use commons::executor::ThreadPool;

use super::*;

use crate::game::traits::{GetRoute, HasWorld, Nations, Settlements, WhoControlsTile};
use crate::traits::RemoveWorldObject;
use std::collections::HashSet;

const NAME: &str = "refresh_positions";
const BATCH_SIZE: usize = 128;

pub struct RefreshPositions<G, X>
where
    G: Send,
{
    game: FnSender<G>,
    x: X,
    pool: ThreadPool,
}

#[async_trait]
impl<G, X> Processor for RefreshPositions<G, X>
where
    G: GetRoute + HasWorld + Nations + Settlements + WhoControlsTile + Send,
    X: RemoveWorldObject + Clone + Send + Sync + 'static,
{
    async fn process(&mut self, state: State, instruction: &Instruction) -> State {
        let positions = match instruction {
            Instruction::RefreshPositions(positions) => positions.clone(),
            _ => return state,
        };
        self.refresh_positions_in_batches(state, positions).await
    }
}

impl<G, X> RefreshPositions<G, X>
where
    G: GetRoute + HasWorld + Nations + Settlements + WhoControlsTile + Send,
    X: RemoveWorldObject + Clone + Send + Sync + 'static,
{
    pub fn new(game: &FnSender<G>, x: X, pool: ThreadPool) -> RefreshPositions<G, X> {
        RefreshPositions {
            game: game.clone_with_name(NAME),
            x,
            pool,
        }
    }

    async fn refresh_positions_in_batches(
        &mut self,
        mut state: State,
        positions: HashSet<V2<usize>>,
    ) -> State {
        let positions: Vec<V2<usize>> = positions.into_iter().collect();
        for batch in positions.chunks(BATCH_SIZE) {
            state = self.refresh_positions(state, batch.to_vec()).await;
        }
        state
    }

    async fn refresh_positions(&mut self, state: State, positions: Vec<V2<usize>>) -> State {
        let initial_town_population = state.params.initial_town_population;
        let x = self.x.clone();
        let pool = self.pool.clone();
        self.game
            .send(move |game| {
                refresh_positions(game, x, pool, state, positions, initial_town_population)
            })
            .await
    }
}

fn refresh_positions<G, X>(
    game: &mut G,
    x: X,
    pool: ThreadPool,
    mut state: State,
    positions: Vec<V2<usize>>,
    initial_town_population: f64,
) -> State
where
    G: GetRoute + HasWorld + Nations + Settlements + WhoControlsTile + Send,
    X: RemoveWorldObject + Clone + Send + Sync + 'static,
{
    for position in positions {
        refresh_position(
            game,
            &x,
            &pool,
            &mut state,
            position,
            &initial_town_population,
        );
    }
    state
}

fn refresh_position<G, X>(
    game: &mut G,
    x: &X,
    pool: &ThreadPool,
    state: &mut State,
    position: V2<usize>,
    initial_town_population: &f64,
) where
    G: GetRoute + HasWorld + Nations + Settlements + WhoControlsTile + Send,
    X: RemoveWorldObject + Clone + Send + Sync + 'static,
{
    let traffic = get_position_traffic(game, &state, &position);
    for instruction in try_build_town(game, &traffic, &initial_town_population) {
        state.build_queue.insert(instruction);
    }
    if let Some(instruction) = try_build_crops(game, &traffic) {
        state.build_queue.insert(instruction);
    }
    try_remove_crops(state, game, x, pool, &traffic);
}
