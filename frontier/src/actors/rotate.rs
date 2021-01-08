use std::sync::mpsc::Sender;
use std::sync::Arc;

use commons::async_trait::async_trait;
use isometric::event_handlers::RotateHandler;
use isometric::{Command, Event, EventHandler, VirtualKeyCode};

use crate::system::HandleEngineEvent;

pub struct Rotate {
    command_tx: Sender<Vec<Command>>,
    engine_rotatehandler: RotateHandler,
}

impl Rotate {
    pub fn new(command_tx: Sender<Vec<Command>>) -> Rotate {
        Rotate {
            command_tx,
            engine_rotatehandler: RotateHandler::new(VirtualKeyCode::Q, VirtualKeyCode::E),
        }
    }

    pub fn set_rotate_over_undrawn(&mut self, on: bool) {
        if on {
            self.engine_rotatehandler.rotate_over_undrawn();
        } else {
            self.engine_rotatehandler.no_rotate_over_undrawn();
        }
    }
}

#[async_trait]
impl HandleEngineEvent for Rotate {
    async fn handle_engine_event(&mut self, event: Arc<Event>) {
        let commands = self.engine_rotatehandler.handle_event(event);
        self.command_tx.send(commands).unwrap();
    }
}