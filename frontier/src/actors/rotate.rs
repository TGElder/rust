use std::sync::Arc;

use commons::async_trait::async_trait;
use isometric::event_handlers::RotateHandler;
use isometric::{Event, EventHandler, VirtualKeyCode};

use crate::system::{Capture, HandleEngineEvent};
use crate::traits::SendEngineCommands;

pub struct Rotate<T> {
    cx: T,
    engine_rotatehandler: RotateHandler,
}

impl<T> Rotate<T>
where
    T: SendEngineCommands,
{
    pub fn new(cx: T) -> Rotate<T> {
        Rotate {
            cx,
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
impl<T> HandleEngineEvent for Rotate<T>
where
    T: SendEngineCommands + Send + Sync,
{
    async fn handle_engine_event(&mut self, event: Arc<Event>) -> Capture {
        let commands = self.engine_rotatehandler.handle_event(event);
        self.cx.send_engine_commands(commands).await;
        Capture::No
    }
}
