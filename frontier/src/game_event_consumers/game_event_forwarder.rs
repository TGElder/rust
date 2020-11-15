use super::*;

use commons::async_channel::{unbounded, Receiver, Sender};
use isometric::Event;
use std::sync::Arc;

const NAME: &str = "game_event_forwarder";

pub struct GameEventForwarder {
    subscribers: Vec<Sender<GameEvent>>,
    pool: ThreadPool,
}

impl GameEventForwarder {
    pub fn new(pool: ThreadPool) -> GameEventForwarder {
        GameEventForwarder {
            subscribers: vec![],
            pool,
        }
    }

    pub fn subscribe(&mut self) -> Receiver<GameEvent> {
        let (tx, rx) = unbounded();
        self.subscribers.push(tx);
        rx
    }

    fn send_event(&mut self, event: &dyn Fn() -> GameEvent) {
        for subscriber in self.subscribers.iter_mut() {
            let subscriber = subscriber.clone();
            let event = event();
            self.pool
                .spawn_ok(async move { subscriber.send(event).await.unwrap() });
        }
    }
}

impl GameEventConsumer for GameEventForwarder {
    fn name(&self) -> &'static str {
        NAME
    }

    #[allow(clippy::single_match)]
    fn consume_game_event(&mut self, _: &GameState, event: &GameEvent) -> CaptureEvent {
        match event {
            GameEvent::NewGame => self.send_event(&|| GameEvent::NewGame),
            GameEvent::Init => self.send_event(&|| GameEvent::Init),
            GameEvent::Save(path) => self.send_event(&|| GameEvent::Save(path.clone())),
            GameEvent::Load(path) => self.send_event(&|| GameEvent::Load(path.clone())),
            _ => (),
        };
        CaptureEvent::No
    }

    fn consume_engine_event(&mut self, _: &GameState, _: Arc<Event>) -> CaptureEvent {
        CaptureEvent::No
    }
}
