use super::*;

use crate::settlement::*;
use commons::grid::Grid;

const HANDLE: &str = "homeland_population_sim";

pub struct HomelandPopulationSim {
    game_tx: UpdateSender<Game>,
}

impl Step for HomelandPopulationSim {
    fn name(&self) -> &'static str {
        HANDLE
    }

    fn init(&mut self) {}

    fn step(&mut self, _: u128) {
        block_on(self.step_async())
    }
}

impl HomelandPopulationSim {
    pub fn new(game_tx: &UpdateSender<Game>) -> HomelandPopulationSim {
        HomelandPopulationSim {
            game_tx: game_tx.clone_with_handle(HANDLE),
        }
    }

    async fn step_async(&mut self) {
        let homelands = self.get_homelands().await;
        let visible_cells = self.count_visible().await;
        for homeland in homelands {
            self.set_target_population(homeland, visible_cells as f64)
                .await
        }
    }

    async fn world_width(&mut self) -> usize {
        self.game_tx.update(|game| world_width(game)).await
    }

    async fn count_visible(&mut self) -> usize {
        let width = self.world_width().await;
        let mut visible = 0;
        for x in 0..width {
            visible += self
                .game_tx
                .update(move |game| count_visible(game, x))
                .await;
        }
        visible
    }

    async fn get_homelands(&mut self) -> Vec<V2<usize>> {
        self.game_tx.update(move |game| get_homelands(game)).await
    }

    async fn set_target_population(&mut self, homeland: V2<usize>, target_population: f64) {
        self.game_tx
            .update(move |game| set_target_population(game, homeland, target_population))
            .await
    }
}

fn get_homelands(game: &mut Game) -> Vec<V2<usize>> {
    game.game_state()
        .settlements
        .values()
        .filter(|settlement| is_homeland(settlement))
        .map(|settlement| settlement.position)
        .collect()
}

fn is_homeland(settlement: &Settlement) -> bool {
    if let SettlementClass::Homeland = settlement.class {
        true
    } else {
        false
    }
}

fn world_width(game: &mut Game) -> usize {
    game.game_state().world.width()
}

fn count_visible(game: &mut Game, x: usize) -> usize {
    let world = &game.game_state().world;
    (0..world.height())
        .map(|y| v2(x, y))
        .filter(|position| is_visible(&world, position))
        .count()
}

fn is_visible(world: &World, position: &V2<usize>) -> bool {
    if let Some(WorldCell { visible: true, .. }) = world.get_cell(position) {
        !world.is_sea(position)
    } else {
        false
    }
}

fn set_target_population(game: &mut Game, settlement: V2<usize>, target_population: f64) {
    let settlement = unwrap_or!(game.game_state().settlements.get(&settlement), return);
    let updated_settlement = Settlement {
        target_population,
        ..*settlement
    };
    game.update_settlement(updated_settlement);
}
