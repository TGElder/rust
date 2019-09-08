use engine::{Command, Event};

use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;
use std::thread;

pub trait EventHandler: Send {
    fn handle_event(&mut self, event: Arc<Event>) -> Vec<Command>;
}

pub trait EventConsumer: Send {
    fn consume_event(&mut self, event: Arc<Event>);
}

pub struct EventHandlerAdapter {
    pub event_handler: Box<EventHandler>,
    pub command_tx: Sender<Vec<Command>>,
}

impl EventConsumer for EventHandlerAdapter {
    fn consume_event(&mut self, event: Arc<Event>) {
        self.command_tx
            .send(self.event_handler.handle_event(event))
            .expect("EventHandlerEventConsumer lost connection to command sender.");
    }
}

pub struct AsyncEventConsumer {
    event_tx: Sender<Arc<Event>>,
}

impl AsyncEventConsumer {
    pub fn new<T>(mut event_consumer: T) -> AsyncEventConsumer
    where
        T: EventConsumer + Send + 'static,
    {
        let (event_tx, event_rx): (Sender<Arc<Event>>, Receiver<Arc<Event>>) = mpsc::channel();

        thread::spawn(move || loop {
            match event_rx.recv() {
                Ok(event) => {
                    event_consumer.consume_event(event.clone());
                    if let Event::Shutdown = *event {
                        return;
                    }
                }
                Err(err) => panic!("Actor could not receive message: {:?}", err),
            }
        });
        AsyncEventConsumer { event_tx }
    }

    fn send_event(&mut self, event: Arc<Event>) {
        match self.event_tx.send(event) {
            Ok(_) => (),
            _ => panic!("Event receiver in AsyncEventHandler hung up!"),
        }
    }
}

impl EventConsumer for AsyncEventConsumer {
    fn consume_event(&mut self, event: Arc<Event>) {
        self.send_event(event);
    }
}
