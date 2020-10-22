use super::*;

use commons::async_channel::{unbounded, Receiver, Sender};
use commons::futures::executor::block_on;
use isometric::Event;
use std::sync::Arc;

const HANDLE: &str = "game_event_forwarder";

pub struct GameEventForwarder {
    subscribers: Vec<Sender<GameEvent>>,
}

impl GameEventForwarder {
    pub fn new() -> GameEventForwarder {
        GameEventForwarder {
            subscribers: vec![],
        }
    }

    pub fn subscribe(&mut self) -> Receiver<GameEvent> {
        let (tx, rx) = unbounded();
        self.subscribers.push(tx);
        rx
    }

    fn send_event(&mut self, event: &dyn Fn() -> GameEvent) {
        for subscriber in self.subscribers.iter_mut() {
            block_on(subscriber.send(event())).unwrap();
        }
    }
}

impl GameEventConsumer for GameEventForwarder {
    fn name(&self) -> &'static str {
        HANDLE
    }

    #[allow(clippy::single_match)]
    fn consume_game_event(&mut self, _: &GameState, event: &GameEvent) -> CaptureEvent {
        match event {
            GameEvent::Init => self.send_event(&|| GameEvent::Init),
            _ => (),
        };
        CaptureEvent::No
    }

    fn consume_engine_event(&mut self, _: &GameState, _: Arc<Event>) -> CaptureEvent {
        CaptureEvent::No
    }
}
