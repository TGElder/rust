use coords::GlCoord4D;
use engine::{Command, Event};
use events::EventHandler;
use std::sync::Arc;

#[derive(Default)]
pub struct DragHandler {
    dragging: bool,
    last_pos: Option<GlCoord4D>,
}

impl DragHandler {
    fn handle_mouse_state(&mut self, state: glutin::event::ElementState) -> Vec<Command> {
        match state {
            glutin::event::ElementState::Pressed => self.dragging = true,
            glutin::event::ElementState::Released => self.dragging = false,
        };
        vec![]
    }

    fn handle_cursor_moved(&mut self, position: Option<GlCoord4D>) -> Vec<Command> {
        let out = if self.dragging {
            if let (Some(last_pos), Some(position)) = (self.last_pos, position) {
                let drag = GlCoord4D::new(
                    position.x - last_pos.x,
                    position.y - last_pos.y,
                    position.z - last_pos.z,
                    position.w - last_pos.w,
                );
                vec![Command::Event(Event::Drag(drag))]
            } else {
                vec![]
            }
        } else {
            vec![]
        };
        self.last_pos = position;
        out
    }
}

impl EventHandler for DragHandler {
    fn handle_event(&mut self, event: Arc<Event>) -> Vec<Command> {
        match *event {
            Event::GlutinEvent(glutin::event::Event::WindowEvent {
                event:
                    glutin::event::WindowEvent::MouseInput {
                        state,
                        button: glutin::event::MouseButton::Left,
                        ..
                    },
                ..
            }) => self.handle_mouse_state(state),
            Event::CursorMoved(gl_position) => self.handle_cursor_moved(gl_position),
            _ => vec![],
        }
    }
}
