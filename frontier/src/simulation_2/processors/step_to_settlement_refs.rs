use super::*;
use crate::game::traits::Settlements;
use crate::settlement::Settlement;

const HANDLE: &str = "step_to_settlement_refs";

pub struct StepToSettlementRefs<T>
where
    T: Settlements,
{
    tx: UpdateSender<T>,
}

impl<T> Processor for StepToSettlementRefs<T>
where
    T: Settlements,
{
    fn process(&mut self, state: State, instruction: &Instruction) -> State {
        block_on(self.process(state, instruction))
    }
}

impl<T> StepToSettlementRefs<T>
where
    T: Settlements,
{
    pub fn new(tx: &UpdateSender<T>) -> StepToSettlementRefs<T> {
        StepToSettlementRefs {
            tx: tx.clone_with_handle(HANDLE),
        }
    }

    pub async fn process(&mut self, mut state: State, instruction: &Instruction) -> State {
        match instruction {
            Instruction::Step => (),
            _ => return state,
        };
        for position in self.get_settlement_positions().await {
            state
                .instructions
                .push(Instruction::SettlementRef(position));
        }
        State { ..state }
    }

    async fn get_settlement_positions(&mut self) -> Vec<V2<usize>> {
        self.tx
            .update(|settlements| get_settlement_positions(settlements))
            .await
    }
}

fn get_settlement_positions(settlements: &dyn Settlements) -> Vec<V2<usize>> {
    settlements
        .settlements()
        .values()
        .map(|Settlement { position, .. }| *position)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    use commons::update::{process_updates, update_channel};
    use commons::{same_elements, v2};
    use std::collections::HashMap;
    use std::thread;

    fn settlement(position: V2<usize>) -> Settlement {
        Settlement {
            position,
            ..Settlement::default()
        }
    }

    #[test]
    fn should_add_instruction_for_each_settlement() {
        let (tx, mut rx) = update_channel(100);

        let handle = thread::spawn(move || {
            let mut settlements = HashMap::new();
            settlements.insert(v2(1, 1), settlement(v2(1, 1)));
            settlements.insert(v2(2, 2), settlement(v2(2, 2)));
            loop {
                let updates = rx.get_updates();
                if !updates.is_empty() {
                    process_updates(updates, &mut settlements);
                    return;
                }
            }
        });

        let mut processor = StepToSettlementRefs::new(&tx);
        let state = block_on(async {
            processor
                .process(State::default(), &Instruction::Step)
                .await
        });
        same_elements(
            &state.instructions,
            &[
                Instruction::SettlementRef(v2(1, 1)),
                Instruction::SettlementRef(v2(2, 2)),
            ],
        );
        handle.join().unwrap();
    }
}
