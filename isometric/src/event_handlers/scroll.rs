use coords::{GlCoord2D, GlCoord4D};
use engine::{Command, Event};
use events::EventHandler;
use std::sync::Arc;

#[derive(Default)]
pub struct Scroller {}

impl EventHandler for Scroller {
    fn handle_event(&mut self, event: Arc<Event>) -> Vec<Command> {
        match *event {
            Event::Drag(GlCoord4D { x, y, .. }) => vec![Command::Translate(GlCoord2D { x, y })],
            _ => vec![],
        }
    }
}
