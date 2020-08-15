use super::*;

use crate::game::traits::{GetRoute, HasWorld, Micros, Nations, Settlements, WhoControlsTile};
use std::collections::HashSet;

const HANDLE: &str = "refresh_positions";
const BATCH_SIZE: usize = 128;

pub struct RefreshPositions<G>
where
    G: GetRoute + HasWorld + Micros + Nations + Settlements + WhoControlsTile,
{
    game: UpdateSender<G>,
}

impl<G> Processor for RefreshPositions<G>
where
    G: GetRoute + HasWorld + Micros + Nations + Settlements + WhoControlsTile,
{
    fn process(&mut self, state: State, instruction: &Instruction) -> State {
        let positions = match instruction {
            Instruction::RefreshPositions(positions) => positions.clone(),
            _ => return state,
        };
        self.refresh_positions_in_batches(state, positions)
    }
}

impl<G> RefreshPositions<G>
where
    G: GetRoute + HasWorld + Micros + Nations + Settlements + WhoControlsTile,
{
    pub fn new(game: &UpdateSender<G>) -> RefreshPositions<G> {
        RefreshPositions {
            game: game.clone_with_handle(HANDLE),
        }
    }

    fn refresh_positions_in_batches(
        &mut self,
        mut state: State,
        positions: HashSet<V2<usize>>,
    ) -> State {
        let positions: Vec<V2<usize>> = positions.into_iter().collect();
        for batch in positions.chunks(BATCH_SIZE) {
            state = self.refresh_positions(state, batch.to_vec());
        }
        state
    }

    fn refresh_positions(&mut self, state: State, positions: Vec<V2<usize>>) -> State {
        block_on(async {
            self.game
                .update(move |game| refresh_positions(game, state, positions))
                .await
        })
    }
}

fn refresh_positions<G>(game: &mut G, mut state: State, positions: Vec<V2<usize>>) -> State
where
    G: GetRoute + HasWorld + Micros + Nations + Settlements + WhoControlsTile,
{
    for position in positions {
        refresh_position(game, &mut state, position);
    }
    state
}

fn refresh_position<G>(game: &mut G, state: &mut State, position: V2<usize>)
where
    G: GetRoute + HasWorld + Micros + Nations + Settlements + WhoControlsTile,
{
    let traffic = get_position_traffic(game, &state, &position);
    if let Some(instruction) = try_build_town(game, &traffic) {
        state.build_queue.push(instruction);
    }
    if let Some(instruction) = try_build_crops(game, &traffic) {
        state.build_queue.push(instruction);
    }
}
