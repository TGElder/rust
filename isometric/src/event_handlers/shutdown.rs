use engine::{Command, Event};
use events::EventHandler;
use std::sync::Arc;

pub struct ShutdownHandler {}

impl ShutdownHandler {
    pub fn new() -> ShutdownHandler {
        ShutdownHandler {}
    }
}

impl EventHandler for ShutdownHandler {
    fn handle_event(&mut self, event: Arc<Event>) -> Vec<Command> {
        match *event {
            Event::GlutinEvent(glutin::Event::WindowEvent {
                event: glutin::WindowEvent::CloseRequested,
                ..
            }) => vec![Command::Shutdown],
            _ => vec![],
        }
    }
}
