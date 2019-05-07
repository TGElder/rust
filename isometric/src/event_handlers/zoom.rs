use coords::{GLCoord2D, GLCoord4D};
use engine::{Command, Event};
use events::EventHandler;
use std::sync::Arc;
use {ElementState, VirtualKeyCode};

pub struct ZoomHandler {
    cursor_position: Option<GLCoord4D>,
}

impl ZoomHandler {
    pub fn new() -> ZoomHandler {
        ZoomHandler {
            cursor_position: None,
        }
    }

    fn zoom(&self, delta: f32) -> Vec<Command> {
        if let Some(center) = self.cursor_position {
            vec![Command::Scale {
                center,
                scale: GLCoord2D::new(delta, delta),
            }]
        } else {
            vec![]
        }
    }

    fn handle_mouse_scroll_delta(&self, delta: glutin::MouseScrollDelta) -> Vec<Command> {
        match delta {
            glutin::MouseScrollDelta::LineDelta(_, d) if d > 0.0 => self.zoom(2.0),
            glutin::MouseScrollDelta::LineDelta(_, d) if d < 0.0 => self.zoom(0.5),
            _ => vec![],
        }
    }
}

impl EventHandler for ZoomHandler {
    fn handle_event(&mut self, event: Arc<Event>) -> Vec<Command> {
        match *event {
            Event::GlutinEvent(glutin::Event::WindowEvent {
                event: glutin::WindowEvent::MouseWheel { delta, .. },
                ..
            }) => self.handle_mouse_scroll_delta(delta),

            Event::Key {
                key: VirtualKeyCode::Add,
                state: ElementState::Pressed,
                ..
            } => self.zoom(2.0),
            Event::Key {
                key: VirtualKeyCode::Subtract,
                state: ElementState::Pressed,
                ..
            } => self.zoom(0.5),
            Event::CursorMoved(gl_position) => {
                self.cursor_position = Some(gl_position);
                vec![]
            }
            _ => vec![],
        }
    }
}
