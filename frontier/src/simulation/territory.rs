use super::*;

use commons::grid::get_corners;
use std::collections::HashSet;
use std::time::Duration;

const HANDLE: &str = "territory_sim";

pub struct TerritorySim {
    game_tx: UpdateSender<Game>,
    pathfinder_tx: UpdateSender<Pathfinder<AvatarTravelDuration>>,
    duration: Duration,
}

impl Step for TerritorySim {
    fn step(&mut self, _: u128) {
        block_on(self.step_async())
    }
}

impl TerritorySim {
    pub fn new(
        game_tx: &UpdateSender<Game>,
        pathfinder_tx: &UpdateSender<Pathfinder<AvatarTravelDuration>>,
        duration: Duration,
    ) -> TerritorySim {
        TerritorySim {
            game_tx: game_tx.clone_with_handle(HANDLE),
            pathfinder_tx: pathfinder_tx.clone_with_handle(HANDLE),
            duration,
        }
    }

    pub fn clone(&self) -> TerritorySim {
        TerritorySim::new(&self.game_tx, &self.pathfinder_tx, self.duration)
    }

    async fn step_async(&mut self) {
        for controller in self.get_controllers().await {
            self.step_controller(controller).await
        }
    }

    async fn get_controllers(&mut self) -> HashSet<V2<usize>> {
        self.game_tx
            .update(|game| game.game_state().territory.controllers())
            .await
    }

    pub async fn step_controller(&mut self, controller: V2<usize>) {
        let corners = get_corners(&controller);
        let duration = self.duration;
        let durations = self
            .pathfinder_tx
            .update(move |pathfinder| pathfinder.positions_within(&corners, duration))
            .await;
        let states = vec![TerritoryState {
            controller,
            durations,
        }];
        self.game_tx
            .update(move |game| game.set_territory(states))
            .await;
    }
}