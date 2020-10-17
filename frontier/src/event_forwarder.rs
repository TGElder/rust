use commons::async_channel::{unbounded, Receiver, Sender};
use commons::futures::executor::block_on;
use isometric::{Event, EventConsumer};
use std::sync::Arc;

pub struct EventForwarder {
    subscribers: Vec<Sender<Arc<Event>>>,
}

impl EventForwarder {
    pub fn new() -> EventForwarder {
        EventForwarder {
            subscribers: vec![],
        }
    }

    pub fn subscribe(&mut self) -> Receiver<Arc<Event>> {
        let (tx, rx) = unbounded();
        self.subscribers.push(tx);
        rx
    }
}

impl EventConsumer for EventForwarder {
    fn consume_event(&mut self, event: Arc<Event>) {
        for subscriber in self.subscribers.iter_mut() {
            block_on(async { subscriber.send(event.clone()).await }).unwrap();
        }
    }
}
