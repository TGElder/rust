use coords::GLCoord4D;
use engine::{Command, Event};
use events::EventHandler;
use std::f32::consts::PI;
use std::sync::Arc;
use {ElementState, VirtualKeyCode};

const DELTA: f32 = PI / 4.0;

pub struct RotateHandler {
    cursor_position: Option<GLCoord4D>,
    clockwise_key: VirtualKeyCode,
    anticlockwise_key: VirtualKeyCode,
}

impl RotateHandler {
    pub fn new(clockwise_key: VirtualKeyCode, anticlockwise_key: VirtualKeyCode) -> RotateHandler {
        RotateHandler {
            cursor_position: None,
            clockwise_key,
            anticlockwise_key,
        }
    }

    fn handle_key(&self, key: VirtualKeyCode) -> Vec<Command> {
        if let Some(center) = self.cursor_position {
            if key == self.clockwise_key {
                vec![Command::Rotate { center, yaw: DELTA }]
            } else if key == self.anticlockwise_key {
                vec![Command::Rotate {
                    center,
                    yaw: -DELTA,
                }]
            } else {
                vec![]
            }
        } else {
            vec![]
        }
    }
}

impl EventHandler for RotateHandler {
    fn handle_event(&mut self, event: Arc<Event>) -> Vec<Command> {
        match *event {
            Event::Key {
                key,
                state: ElementState::Pressed,
                ..
            } => self.handle_key(key),
            Event::CursorMoved(gl_position) => {
                self.cursor_position = Some(gl_position);
                vec![]
            }
            _ => vec![],
        }
    }
}
