use super::*;

use commons::grid::get_corners;
use std::collections::HashSet;
use std::sync::RwLock;
use std::time::Duration;

const HANDLE: &str = "territory_sim";

pub struct TerritorySim {
    game_tx: UpdateSender<Game>,
    pathfinder: Arc<RwLock<Pathfinder<AvatarTravelDuration>>>,
    duration: Duration,
}

impl Step for TerritorySim {
    fn name(&self) -> &'static str {
        HANDLE
    }

    fn init(&mut self) {}

    fn step(&mut self, _: u128) {
        block_on(self.step_async())
    }
}

impl TerritorySim {
    pub fn new(
        game_tx: &UpdateSender<Game>,
        pathfinder: &Arc<RwLock<Pathfinder<AvatarTravelDuration>>>,
        duration: Duration,
    ) -> TerritorySim {
        TerritorySim {
            game_tx: game_tx.clone_with_handle(HANDLE),
            pathfinder: pathfinder.clone(),
            duration,
        }
    }

    pub fn clone(&self) -> TerritorySim {
        TerritorySim::new(&self.game_tx, &self.pathfinder, self.duration)
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
            .pathfinder
            .read()
            .unwrap()
            .positions_within(&corners, duration);
        let states = vec![TerritoryState {
            controller,
            durations,
        }];
        self.game_tx
            .update(move |game| game.set_territory(states))
            .await;
    }
}
