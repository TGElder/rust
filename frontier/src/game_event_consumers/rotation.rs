use super::*;
use isometric::event_handlers::RotateHandler as EngineRotateHandler;
use isometric::{EventHandler, VirtualKeyCode};

const HANDLE: &str = "rotate_handler";

pub struct RotateHandler {
    game_tx: FnSender<Game>,
    engine_rotatehandler: EngineRotateHandler,
}

impl RotateHandler {
    pub fn new(game_tx: &FnSender<Game>) -> RotateHandler {
        RotateHandler {
            game_tx: game_tx.clone_with_name(HANDLE),
            engine_rotatehandler: EngineRotateHandler::new(VirtualKeyCode::Q, VirtualKeyCode::E),
        }
    }
}

impl GameEventConsumer for RotateHandler {
    fn name(&self) -> &'static str {
        HANDLE
    }

    fn consume_game_event(&mut self, _: &GameState, _: &GameEvent) -> CaptureEvent {
        CaptureEvent::No
    }

    fn consume_engine_event(&mut self, game_state: &GameState, event: Arc<Event>) -> CaptureEvent {
        if game_state.follow_avatar {
            self.engine_rotatehandler.rotate_over_undrawn();
        } else {
            self.engine_rotatehandler.no_rotate_over_undrawn();
        }
        let commands = self.engine_rotatehandler.handle_event(event);
        if !commands.is_empty() {
            self.game_tx
                .send(move |game| game.send_engine_commands(commands));
        }
        CaptureEvent::No
    }
}
