use coords::GLCoord4D;
use engine::{Button, Command, Event};
use events::EventHandler;
use std::f32::consts::PI;
use std::sync::Arc;
use {ElementState, VirtualKeyCode};

const DELTA: f32 = PI / 16.0;

pub struct RotateHandler {
    cursor_position: Option<GLCoord4D>,
    clockwise_key: VirtualKeyCode,
    anticlockwise_key: VirtualKeyCode,
    rotate_over_undrawn: bool,
}

impl RotateHandler {
    pub fn new(clockwise_key: VirtualKeyCode, anticlockwise_key: VirtualKeyCode) -> RotateHandler {
        RotateHandler {
            cursor_position: None,
            clockwise_key,
            anticlockwise_key,
            rotate_over_undrawn: true,
        }
    }

    pub fn rotate_over_undrawn(&mut self) {
        self.rotate_over_undrawn = true;
    }

    pub fn no_rotate_over_undrawn(&mut self) {
        self.rotate_over_undrawn = false;
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
            Event::Button {
                button: Button::Key(key),
                state: ElementState::Pressed,
                ..
            } => self.handle_key(key),
            Event::CursorMoved(Some(gl_position)) => {
                self.cursor_position = if self.rotate_over_undrawn || gl_position.z < 1.0 {
                    Some(gl_position)
                } else {
                    None // Nothing drawn here
                };
                vec![]
            }
            _ => vec![],
        }
    }
}
