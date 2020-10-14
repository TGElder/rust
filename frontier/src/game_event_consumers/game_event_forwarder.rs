use super::*;
use isometric::EventHandler;

use commons::async_channel::{unbounded, Receiver, Sender};
use commons::futures::executor::block_on;
use crate::game::GameEvent;
use std::sync::Arc;

pub struct EventForwarder {
    tx: Sender<Arc<Event>>,
    rx: Receiver<Arc<Event>>,
}

impl EventForwarder {
    pub fn new() -> EventForwarder {
        let (tx, rx) = unbounded();
        EventForwarder { tx, rx }
    }

    pub fn rx(&self) -> &Receiver<Arc<Event>> {
        &self.rx
    }
}

impl<T> GameEventConsumer for EventHandlerAdapter<T>
where
    T: EventHandler,
{
    fn name(&self) -> &'static str {
        HANDLE
    }

    fn consume_game_event(&mut self, _: &GameState, event: &GameEvent) -> CaptureEvent {
        block_on(self.tx.send(event)).unwrap();
        CaptureEvent::No
    }

    fn consume_engine_event(&mut self, _: &GameState, event: Arc<Event>) -> CaptureEvent {
        CaptureEvent::No
    }
}