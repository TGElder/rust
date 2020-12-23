use engine::{Button, Command, Event};
use events::EventHandler;
use std::sync::Arc;

#[derive(Default)]
pub struct KeyRelay {}

impl EventHandler for KeyRelay {

    #[allow(deprecated)]
    fn handle_event(&mut self, event: Arc<Event>) -> Vec<Command> {
        match *event {
            Event::GlutinEvent(glutin::event::Event::WindowEvent {
                event:
                    glutin::event::WindowEvent::KeyboardInput {
                        input:
                            glutin::event::KeyboardInput {
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
                    vec![Command::Event(Event::Button {
                        button: Button::Key(key),
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
