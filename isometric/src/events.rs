use commons::async_channel::Sender;
use engine::{Command, Event};
use futures::executor::block_on;

use std::sync::Arc;

pub trait EventHandler: Send {
    fn handle_event(&mut self, event: Arc<Event>) -> Vec<Command>;
}

pub trait EventConsumer: Send {
    fn consume_event(&mut self, event: Arc<Event>);
}

pub struct EventHandlerAdapter {
    pub event_handler: Box<dyn EventHandler>,
    pub command_tx: Sender<Vec<Command>>,
}

impl EventConsumer for EventHandlerAdapter {
    fn consume_event(&mut self, event: Arc<Event>) {
        block_on(self.command_tx.send(self.event_handler.handle_event(event)))
            .expect("EventHandlerEventConsumer lost connection to command sender.");
    }
}
