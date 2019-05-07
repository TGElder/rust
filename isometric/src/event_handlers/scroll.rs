use coords::{GLCoord2D, GLCoord4D};
use engine::{Command, Event};
use events::EventHandler;
use std::sync::Arc;

pub struct Scroller {}

impl Scroller {
    pub fn new() -> Scroller {
        Scroller {}
    }
}

impl EventHandler for Scroller {
    fn handle_event(&mut self, event: Arc<Event>) -> Vec<Command> {
        match *event {
            Event::Drag(GLCoord4D { x, y, .. }) => vec![Command::Translate(GLCoord2D { x, y })],
            _ => vec![],
        }
    }
}
