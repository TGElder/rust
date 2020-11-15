use super::*;

use crate::simulation::Simulation;

const NAME: &str = "shutdown_handler";

pub struct ShutdownHandler {
    game_tx: FnSender<Game>,
    sim_tx: FnSender<Simulation>,
    pool: ThreadPool,
}

impl ShutdownHandler {
    pub fn new(
        game_tx: &FnSender<Game>,
        sim_tx: &FnSender<Simulation>,
        pool: ThreadPool,
    ) -> ShutdownHandler {
        ShutdownHandler {
            game_tx: game_tx.clone_with_name(NAME),
            sim_tx: sim_tx.clone_with_name(NAME),
            pool,
        }
    }

    fn shutdown(&mut self) {
        let game_tx = self.game_tx.clone();
        let sim_tx = self.sim_tx.clone();
        self.pool.spawn_ok(async move {
            println!("Waiting for simulation to shutdown...");
            sim_tx.send(|sim| sim.shutdown()).await;
            println!("Waiting for game to shutdown...");
            game_tx.send(|game| game.shutdown());
        });
    }
}

impl GameEventConsumer for ShutdownHandler {
    fn name(&self) -> &'static str {
        NAME
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
