use engine::{Command, Event};
use events::EventHandler;
use std::sync::Arc;

pub struct KeyRelay {}

impl KeyRelay {
    pub fn new() -> KeyRelay {
        KeyRelay {}
    }
}

impl EventHandler for KeyRelay {
    fn handle_event(&mut self, event: Arc<Event>) -> Vec<Command> {
        match *event {
            Event::GlutinEvent(glutin::Event::WindowEvent {
                event:
                    glutin::WindowEvent::KeyboardInput {
                        input:
                            glutin::KeyboardInput {
                                virtual_keycode,
                                state,
                                modifiers,
                                ..
                            },
                        ..
                    },
                ..
            }) => {
                if let Some(key) = virtual_keycode {
                    vec![Command::Event(Event::Key {
                        key,
                        state,
                        modifiers,
                    })]
                } else {
                    vec![]
                }
            }
            _ => vec![],
        }
    }
}
