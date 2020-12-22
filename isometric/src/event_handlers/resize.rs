use engine::{Command, Event};
use events::EventHandler;
use std::sync::Arc;

#[derive(Default)]
pub struct Resizer {}

impl EventHandler for Resizer {
    fn handle_event(&mut self, event: Arc<Event>) -> Vec<Command> {
        match *event {
            Event::GlutinEvent(glutin::event::Event::WindowEvent {
                event: glutin::event::WindowEvent::Resized(physical_size),
                ..
            }) => vec![Command::Resize(physical_size)],
            _ => vec![],
        }
    }
}
