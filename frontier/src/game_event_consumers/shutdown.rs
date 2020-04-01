use super::*;

use crate::simulation::*;

const HANDLE: &str = "shutdown_handler";

pub struct ShutdownHandler {
    avatar_pathfinder_tx: UpdateSender<PathfinderService<AvatarTravelDuration>>,
    road_pathfinder_tx: UpdateSender<PathfinderService<AutoRoadTravelDuration>>,
    game_tx: UpdateSender<Game>,
    sim_tx: UpdateSender<Simulation>,
    pool: ThreadPool,
}

impl ShutdownHandler {
    pub fn new(
        avatar_pathfinder_tx: &UpdateSender<PathfinderService<AvatarTravelDuration>>,
        road_pathfinder_tx: &UpdateSender<PathfinderService<AutoRoadTravelDuration>>,
        game_tx: &UpdateSender<Game>,
        sim_tx: &UpdateSender<Simulation>,
        pool: ThreadPool,
    ) -> ShutdownHandler {
        ShutdownHandler {
            avatar_pathfinder_tx: avatar_pathfinder_tx.clone_with_handle(HANDLE),
            road_pathfinder_tx: road_pathfinder_tx.clone_with_handle(HANDLE),
            game_tx: game_tx.clone_with_handle(HANDLE),
            sim_tx: sim_tx.clone_with_handle(HANDLE),
            pool,
        }
    }

    fn shutdown(&mut self) {
        let avatar_pathfinder_tx = self.avatar_pathfinder_tx.clone();
        let road_pathfinder_tx = self.road_pathfinder_tx.clone();
        let game_tx = self.game_tx.clone();
        let sim_tx = self.sim_tx.clone();
        self.pool.spawn_ok(async move {
            println!("Waiting for simulation to shutdown...");
            sim_tx.update(|sim| sim.shutdown()).await;
            println!("Waiting for game to shutdown...");
            game_tx.update(|game| game.shutdown());
            println!("Waiting for road pathfinder to shutdown...");
            road_pathfinder_tx.update(|service| service.shutdown());
            println!("Waiting for avatar pathfinder to shutdown...");
            avatar_pathfinder_tx.update(|service| service.shutdown());
        });
    }
}

impl GameEventConsumer for ShutdownHandler {
    fn name(&self) -> &'static str {
        HANDLE
    }

    fn consume_game_event(&mut self, _: &GameState, _: &GameEvent) -> CaptureEvent {
        CaptureEvent::No
    }

    fn consume_engine_event(&mut self, _: &GameState, event: Arc<Event>) -> CaptureEvent {
        if let Event::Shutdown = *event {
            self.shutdown();
        }
        CaptureEvent::No
    }
}
