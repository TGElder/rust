use super::*;

use crate::traits::{SendGame, SendSim};

const NAME: &str = "shutdown_handler";

pub struct ShutdownHandler<T> {
    x: T,
    pool: ThreadPool,
}

impl<T> ShutdownHandler<T>
where
    T: SendGame + SendSim + Clone + Send + Sync + 'static,
{
    pub fn new(x: T, pool: ThreadPool) -> ShutdownHandler<T> {
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

impl<T> GameEventConsumer for ShutdownHandler<T>
where
    T: SendGame + SendSim + Clone + Send + Sync + 'static,
{
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
