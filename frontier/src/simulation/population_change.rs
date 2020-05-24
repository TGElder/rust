use super::*;
use crate::settlement::Settlement;

const HANDLE: &str = "population_change_sim";

pub struct PopulationChangeSim {
    game_tx: UpdateSender<Game>,
    last_update_micros: u128,
}

impl Step for PopulationChangeSim {
    fn name(&self) -> &'static str {
        HANDLE
    }

    fn init(&mut self) {
        block_on(self.init_async())
    }

    fn step(&mut self, _: u128) {
        block_on(self.step_async())
    }
}

impl PopulationChangeSim {
    pub fn new(game_tx: &UpdateSender<Game>) -> PopulationChangeSim {
        PopulationChangeSim {
            game_tx: game_tx.clone_with_handle(HANDLE),
            last_update_micros: 0,
        }
    }

    async fn init_async(&mut self) {
        self.last_update_micros = self
            .game_tx
            .update(|game| game.game_state().game_micros)
            .await;
    }

    async fn step_async(&mut self) {
        let current_micros = self.get_game_micros().await;
        let gap_seconds = self.get_gap_seconds(current_micros).await;
        for settlement in self.get_settlements().await {
            self.adjust_population(settlement, gap_seconds).await
        }
        self.last_update_micros = current_micros;
    }

    async fn get_gap_seconds(&mut self, current_micros: u128) -> f64 {
        let gap_micros = (current_micros - self.last_update_micros) as f64;
        gap_micros / 1_000_000.0
    }

    async fn get_game_micros(&mut self) -> u128 {
        self.game_tx.update(|game| get_game_micros(game)).await
    }

    async fn get_settlements(&mut self) -> Vec<V2<usize>> {
        self.game_tx.update(|game| get_settlements(game)).await
    }

    async fn adjust_population(&mut self, settlement: V2<usize>, gap_decay: f64) {
        self.game_tx
            .update(move |game| adjust_population(game, settlement, gap_decay))
            .await
    }
}

fn get_game_micros(game: &mut Game) -> u128 {
    game.game_state().game_micros
}

fn get_settlements(game: &mut Game) -> Vec<V2<usize>> {
    game.game_state()
        .settlements
        .values()
        .map(|settlement| settlement.position)
        .collect()
}

fn adjust_population(game: &mut Game, settlement: V2<usize>, gap_seconds: f64) {
    let settlement = unwrap_or!(game.game_state().settlements.get(&settlement), return);
    let gap_half_life = unwrap_or!(settlement.gap_half_life, return);
    let gap_decay = get_gap_decay(&gap_half_life, gap_seconds);
    let gap = settlement.target_population - settlement.current_population;
    let current_population = settlement.target_population - gap * gap_decay;
    let updated_settlement = Settlement {
        current_population,
        ..*settlement
    };
    game.update_settlement(updated_settlement);
}

fn get_gap_decay(gap_half_life: &Duration, gap_seconds: f64) -> f64 {
    let exponent = 1.0 / gap_half_life.as_secs_f64();
    let gap_decay_per_second = 0.5f64.powf(exponent);
    gap_decay_per_second.powf(gap_seconds)
}
