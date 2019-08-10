use coords::GLCoord4D;
use engine::{Command, Event};
use events::EventHandler;
use std::sync::Arc;

#[derive(Default)]
pub struct DragHandler {
    dragging: bool,
    last_pos: Option<GLCoord4D>,
}

impl DragHandler {
    fn handle_mouse_state(&mut self, state: glutin::ElementState) -> Vec<Command> {
        match state {
            glutin::ElementState::Pressed => self.dragging = true,
            glutin::ElementState::Released => self.dragging = false,
        };
        vec![]
    }

    fn handle_cursor_moved(&mut self, position: GLCoord4D) -> Vec<Command> {
        let out = if self.dragging {
            if let Some(last_pos) = self.last_pos {
                let drag = GLCoord4D::new(
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
        self.last_pos = Some(position);
        out
    }
}

impl EventHandler for DragHandler {
    fn handle_event(&mut self, event: Arc<Event>) -> Vec<Command> {
        match *event {
            Event::GlutinEvent(glutin::Event::WindowEvent {
                event:
                    glutin::WindowEvent::MouseInput {
                        state,
                        button: glutin::MouseButton::Left,
                        ..
                    },
                ..
            }) => self.handle_mouse_state(state),
            Event::CursorMoved(gl_position) => self.handle_cursor_moved(gl_position),
            _ => vec![],
        }
    }
}
