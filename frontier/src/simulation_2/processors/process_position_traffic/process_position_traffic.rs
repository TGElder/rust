use super::*;

use crate::game::traits::{GetRoute, HasWorld, Micros, Nations, Settlements, WhoControlsTile};
use std::collections::HashSet;

const HANDLE: &str = "process_position_traffic";
const BATCH_SIZE: usize = 128;

pub struct ProcessPositionTraffic<G>
where
    G: GetRoute + HasWorld + Micros + Nations + Settlements + WhoControlsTile,
{
    game: UpdateSender<G>,
}

impl<G> Processor for ProcessPositionTraffic<G>
where
    G: GetRoute + HasWorld + Micros + Nations + Settlements + WhoControlsTile,
{
    fn process(&mut self, mut state: State, instruction: &Instruction) -> State {
        let route_changes = match instruction {
            Instruction::ProcessRouteChanges(route_changes) => route_changes.clone(),
            _ => return state,
        };
        let position_changes =
            update_all_position_traffic_and_get_changes(&mut state, &route_changes);
        self.process_position_changes_in_batches(state, position_changes)
    }
}

impl<G> ProcessPositionTraffic<G>
where
    G: GetRoute + HasWorld + Micros + Nations + Settlements + WhoControlsTile,
{
    pub fn new(game: &UpdateSender<G>) -> ProcessPositionTraffic<G> {
        ProcessPositionTraffic {
            game: game.clone_with_handle(HANDLE),
        }
    }

    fn process_position_changes_in_batches(
        &mut self,
        mut state: State,
        position_changes: HashSet<V2<usize>>,
    ) -> State {
        let position_changes: Vec<V2<usize>> = position_changes.into_iter().collect();
        for batch in position_changes.chunks(BATCH_SIZE) {
            state = self.process_position_changes(state, batch.to_vec());
        }
        state
    }

    fn process_position_changes(
        &mut self,
        state: State,
        position_changes: Vec<V2<usize>>,
    ) -> State {
        block_on(async {
            self.game
                .update(move |game| process_position_changes(game, state, position_changes))
                .await
        })
    }
}

fn process_position_changes<G>(
    game: &mut G,
    mut state: State,
    position_changes: Vec<V2<usize>>,
) -> State
where
    G: GetRoute + HasWorld + Micros + Nations + Settlements + WhoControlsTile,
{
    for position_change in position_changes {
        process_position_change(game, &mut state, position_change);
    }
    state
}

fn process_position_change<G>(game: &mut G, state: &mut State, position_change: V2<usize>)
where
    G: GetRoute + HasWorld + Micros + Nations + Settlements + WhoControlsTile,
{
    let traffic = get_position_traffic(game, &state, &position_change);
    if let Some(instruction) = try_build_town(game, &traffic) {
        state.build_queue.push(instruction);
    }
    if let Some(instruction) = try_build_crops(game, &traffic) {
        state.build_queue.push(instruction);
    }
}
