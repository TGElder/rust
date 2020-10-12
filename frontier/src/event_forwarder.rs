use commons::async_channel::{Sender, Receiver, unbounded};
use commons::futures::executor::block_on;
use isometric::{Event, EventConsumer};
use std::sync::Arc;

pub struct EventForwarder{
    tx: Sender<Arc<Event>>,
    rx: Receiver<Arc<Event>>,
}


impl EventForwarder{
    pub fn new() -> EventForwarder {
        let (tx, rx) = unbounded();
        EventForwarder{
            tx,
            rx
        }
    }

    pub fn rx(&self) -> &Receiver<Arc<Event>> {
        &self.rx
    }
}


impl EventConsumer for EventForwarder {
    fn consume_event(&mut self, event: Arc<Event>) {
        block_on(self.tx.send(event)).unwrap();
    }
}