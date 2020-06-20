use super::*;
use crate::game::traits::Settlements;
use crate::settlement::{Settlement, SettlementClass};

const HANDLE: &str = "step_to_towns";

pub struct StepToTowns<T>
where
    T: Settlements,
{
    tx: UpdateSender<T>,
}

impl<T> Processor for StepToTowns<T>
where
    T: Settlements,
{
    fn process(&mut self, state: State, instruction: &Instruction) -> State {
        block_on(self.process(state, instruction))
    }
}

impl<T> StepToTowns<T>
where
    T: Settlements,
{
    pub fn new(tx: &UpdateSender<T>) -> StepToTowns<T> {
        StepToTowns {
            tx: tx.clone_with_handle(HANDLE),
        }
    }

    pub async fn process(&mut self, mut state: State, instruction: &Instruction) -> State {
        match instruction {
            Instruction::Step => (),
            _ => return state,
        };
        for position in self.get_town_positions().await {
            state.instructions.push(Instruction::Town(position));
        }
        State { ..state }
    }

    async fn get_town_positions(&mut self) -> Vec<V2<usize>> {
        self.tx
            .update(|settlements| get_town_positions(settlements))
            .await
    }
}

fn get_town_positions(settlements: &dyn Settlements) -> Vec<V2<usize>> {
    settlements
        .settlements()
        .values()
        .filter(|Settlement { class, .. }| *class == SettlementClass::Town)
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

    fn town(position: V2<usize>) -> Settlement {
        Settlement {
            class: SettlementClass::Town,
            position,
            ..Settlement::default()
        }
    }

    fn homeland(position: V2<usize>) -> Settlement {
        Settlement {
            class: SettlementClass::Homeland,
            position,
            ..Settlement::default()
        }
    }

    #[test]
    fn should_add_instruction_for_each_town() {
        let (tx, mut rx) = update_channel(100);

        let handle = thread::spawn(move || {
            let mut settlements = HashMap::new();
            settlements.insert(v2(1, 1), town(v2(1, 1)));
            settlements.insert(v2(1, 1), town(v2(2, 2)));
            loop {
                let updates = rx.get_updates();
                if !updates.is_empty() {
                    process_updates(updates, &mut settlements);
                    return;
                }
            }
        });

        let mut sim = StepToTowns::new(&tx);
        let state = block_on(async { sim.process(State::default(), &Instruction::Step).await });
        same_elements(
            &state.instructions,
            &[Instruction::Town(v2(1, 1)), Instruction::Town(v2(2, 2))],
        );
        handle.join().unwrap();
    }

    #[test]
    fn should_not_add_instructions_for_homelands() {
        let (tx, mut rx) = update_channel(100);

        let handle = thread::spawn(move || {
            let mut settlements = HashMap::new();
            settlements.insert(v2(1, 1), homeland(v2(1, 1)));
            settlements.insert(v2(1, 1), homeland(v2(2, 2)));
            loop {
                let updates = rx.get_updates();
                if !updates.is_empty() {
                    process_updates(updates, &mut settlements);
                    return;
                }
            }
        });

        let mut sim = StepToTowns::new(&tx);
        let state = block_on(async { sim.process(State::default(), &Instruction::Step).await });
        assert!(state.instructions.is_empty());
        handle.join().unwrap();
    }
}
