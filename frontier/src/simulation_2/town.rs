use super::*;
use crate::game::traits::Settlements;

const HANDLE: &str = "sim_town";

pub struct TownSim<T>
where
    T: Settlements,
{
    tx: UpdateSender<T>,
}

impl<T> Processor for TownSim<T>
where
    T: Settlements,
{
    fn process(&mut self, state: State, instruction: &Instruction) -> State {
        block_on(self.process(state, instruction))
    }
}

impl<T> TownSim<T>
where
    T: Settlements,
{
    pub fn new(tx: &UpdateSender<T>) -> TownSim<T> {
        TownSim {
            tx: tx.clone_with_handle(HANDLE),
        }
    }

    pub async fn process(&mut self, state: State, instruction: &Instruction) -> State {
        let town = match instruction {
            Instruction::Town(town) => *town,
            _ => return state,
        };
        let town_name: Option<String> = self
            .tx
            .update(move |settlements| {
                settlements
                    .settlements()
                    .get(&town)
                    .map(|town| town.name.clone())
            })
            .await;
        println!("{:?}", town_name);
        state
    }
}
