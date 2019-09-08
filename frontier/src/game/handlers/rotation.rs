use super::*;
use isometric::event_handlers::RotateHandler as EngineRotateHandler;
use isometric::{EventHandler, VirtualKeyCode};

pub struct RotateHandler {
    command_tx: Sender<GameCommand>,
    engine_rotatehandler: EngineRotateHandler,
}

impl RotateHandler {
    pub fn new(command_tx: Sender<GameCommand>) -> RotateHandler {
        RotateHandler {
            command_tx,
            engine_rotatehandler: EngineRotateHandler::new(VirtualKeyCode::Q, VirtualKeyCode::E),
        }
    }
}

impl GameEventConsumer for RotateHandler {
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
        self.command_tx
            .send(GameCommand::EngineCommands(commands))
            .unwrap();
        CaptureEvent::No
    }
}
