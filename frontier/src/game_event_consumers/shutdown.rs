use super::*;

const HANDLE: &str = "shutdown_handler";

pub struct ShutdownHandler {
    game_tx: UpdateSender<Game>,
}

impl ShutdownHandler {
    pub fn new(game_tx: &UpdateSender<Game>) -> ShutdownHandler {
        ShutdownHandler {
            game_tx: game_tx.clone_with_handle(HANDLE),
        }
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
            self.game_tx.update(|game| game.shutdown());
        }
        CaptureEvent::No
    }

    fn shutdown(&mut self) {}

    fn is_shutdown(&self) -> bool {
        true
    }
}
