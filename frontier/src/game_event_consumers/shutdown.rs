use super::*;

use crate::polysender::Polysender;
use crate::traits::{SendGame, SendSim};

const NAME: &str = "shutdown_handler";

pub struct ShutdownHandler {
    x: Polysender,
    pool: ThreadPool,
}

impl ShutdownHandler {
    pub fn new(x: Polysender, pool: ThreadPool) -> ShutdownHandler {
        ShutdownHandler { x, pool }
    }

    fn shutdown(&mut self) {
        let x = self.x.clone();
        self.pool.spawn_ok(async move {
            println!("Waiting for simulation to shutdown...");
            x.send_sim(|sim| sim.shutdown()).await;
            println!("Waiting for game to shutdown...");
            x.send_game(|game| game.shutdown()).await;
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
