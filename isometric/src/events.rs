use engine::{Command, Event};

use std::sync::mpsc;
use std::sync::mpsc::{Receiver, RecvError, Sender, TryRecvError};
use std::sync::Arc;
use std::thread;

pub trait EventHandler {
    fn handle_event(&mut self, event: Arc<Event>) -> Vec<Command>;
}

pub struct AsyncEventHandler {
    event_tx: Sender<Arc<Event>>,
    command_rx: Receiver<Vec<Command>>,
}

impl AsyncEventHandler {
    pub fn new(mut event_handler: Box<EventHandler + Send>) -> AsyncEventHandler {
        let (event_tx, event_rx) = mpsc::channel();
        let (command_tx, command_rx) = mpsc::channel();

        thread::spawn(move || {
            let send_commands = |commands: Vec<Command>| match command_tx.send(commands) {
                Ok(_) => true,
                _ => panic!("Command receiver in AsyncEventHandler hung up!"),
            };

            let mut handle_event = |event: Arc<Event>| match *event {
                Event::Shutdown => {
                    println!("Shutting down AsyncEventHandler");
                    false
                }
                _ => send_commands(event_handler.handle_event(event)),
            };

            let mut handle_message = |event: Result<Arc<Event>, RecvError>| match event {
                Ok(event) => handle_event(event),
                _ => panic!("Event sender in AsyncEventHandler hung up!"),
            };

            while handle_message(event_rx.recv()) {}
        });
        AsyncEventHandler {
            event_tx,
            command_rx,
        }
    }

    fn send_event(&mut self, event: Arc<Event>) {
        match self.event_tx.send(event) {
            Ok(_) => (),
            _ => panic!("Event receiver in AsyncEventHandler hung up!"),
        }
    }

    fn get_commands(&mut self) -> Vec<Command> {
        let mut out = vec![];
        loop {
            match &mut self.command_rx.try_recv() {
                Ok(commands) => out.append(commands),
                Err(TryRecvError::Empty) => return out,
                Err(TryRecvError::Disconnected) => {
                    panic!("Command sender in AsyncEventHandler hung up!")
                }
            };
        }
    }
}

impl EventHandler for AsyncEventHandler {
    fn handle_event(&mut self, event: Arc<Event>) -> Vec<Command> {
        if let Event::Shutdown = *event {
            self.send_event(event);
            vec![]
        } else {
            self.send_event(event);
            self.get_commands()
        }
    }
}
