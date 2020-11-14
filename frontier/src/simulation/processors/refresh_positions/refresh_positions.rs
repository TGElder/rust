use super::*;

use crate::game::traits::{
    GetRoute, HasWorld, Nations, RemoveObject, Settlements, WhoControlsTile,
};
use std::collections::HashSet;

const HANDLE: &str = "refresh_positions";
const BATCH_SIZE: usize = 128;

pub struct RefreshPositions<G>
where
    G: GetRoute + HasWorld + Nations + RemoveObject + Settlements + WhoControlsTile + Send,
{
    game: FnSender<G>,
}

#[async_trait]
impl<G> Processor for RefreshPositions<G>
where
    G: GetRoute + HasWorld + Nations + RemoveObject + Settlements + WhoControlsTile + Send,
{
    async fn process(&mut self, state: State, instruction: &Instruction) -> State {
        let positions = match instruction {
            Instruction::RefreshPositions(positions) => positions.clone(),
            _ => return state,
        };
        self.refresh_positions_in_batches(state, positions).await
    }
}

impl<G> RefreshPositions<G>
where
    G: GetRoute + HasWorld + Nations + RemoveObject + Settlements + WhoControlsTile + Send,
{
    pub fn new(game: &FnSender<G>) -> RefreshPositions<G> {
        RefreshPositions {
            game: game.clone_with_name(HANDLE),
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
        self.game
            .send(move |game| refresh_positions(game, state, positions, initial_town_population))
            .await
    }
}

fn refresh_positions<G>(
    game: &mut G,
    mut state: State,
    positions: Vec<V2<usize>>,
    initial_town_population: f64,
) -> State
where
    G: GetRoute + HasWorld + Nations + RemoveObject + Settlements + WhoControlsTile + Send,
{
    for position in positions {
        refresh_position(game, &mut state, position, &initial_town_population);
    }
    state
}

fn refresh_position<G>(
    game: &mut G,
    state: &mut State,
    position: V2<usize>,
    initial_town_population: &f64,
) where
    G: GetRoute + HasWorld + Nations + RemoveObject + Settlements + WhoControlsTile + Send,
{
    let traffic = get_position_traffic(game, &state, &position);
    for instruction in try_build_town(game, &traffic, &initial_town_population) {
        state.build_queue.insert(instruction);
    }
    if let Some(instruction) = try_build_crops(game, &traffic) {
        state.build_queue.insert(instruction);
    }
    try_remove_crops(state, game, &traffic);
}
