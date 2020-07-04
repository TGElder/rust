use super::*;

use crate::build_service::BuildService;
use crate::simulation_2::Simulation;

const HANDLE: &str = "shutdown_handler";

pub struct ShutdownHandler {
    game_tx: UpdateSender<Game>,
    builder_tx: UpdateSender<BuildService<Game>>,
    sim_tx: UpdateSender<Simulation>,
    pool: ThreadPool,
}

impl ShutdownHandler {
    pub fn new(
        game_tx: &UpdateSender<Game>,
        builder_tx: &UpdateSender<BuildService<Game>>,
        sim_tx: &UpdateSender<Simulation>,
        pool: ThreadPool,
    ) -> ShutdownHandler {
        ShutdownHandler {
            game_tx: game_tx.clone_with_handle(HANDLE),
            builder_tx: builder_tx.clone_with_handle(HANDLE),
            sim_tx: sim_tx.clone_with_handle(HANDLE),
            pool,
        }
    }

    fn shutdown(&mut self) {
        let game_tx = self.game_tx.clone();
        let builder_tx = self.builder_tx.clone();
        let sim_tx = self.sim_tx.clone();
        self.pool.spawn_ok(async move {
            println!("Waiting for simulation to shutdown...");
            sim_tx.update(|sim| sim.shutdown()).await;
            println!("Waiting for builder to shutdown...");
            builder_tx.update(|builder| builder.shutdown()).await;
            println!("Waiting for game to shutdown...");
            game_tx.update(|game| game.shutdown());
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
