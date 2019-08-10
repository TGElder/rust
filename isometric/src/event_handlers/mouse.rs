use engine::{Command, Event};
use events::EventHandler;
use std::sync::Arc;

#[derive(Default)]
pub struct MouseRelay {}

impl EventHandler for MouseRelay {
    fn handle_event(&mut self, event: Arc<Event>) -> Vec<Command> {
        match *event {
            Event::GlutinEvent(glutin::Event::WindowEvent {
                event: glutin::WindowEvent::MouseInput { button, state, .. },
                ..
            }) => vec![Command::Event(Event::Mouse { button, state })],
            _ => vec![],
        }
    }
}
