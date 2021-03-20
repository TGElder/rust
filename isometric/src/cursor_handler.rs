use coords::{GLCoord4D, PhysicalPositionExt, WorldCoord};
use engine::Event;
use events::EventConsumer;
use glutin::dpi::{PhysicalPosition, PhysicalSize};
use graphics::GLZFinder;
use std::sync::Arc;
use transform::Transform;

pub struct CursorHandler {
    z_finder: GLZFinder,
    physical_window_size: PhysicalSize<u32>,
    screen_cursor: Option<PhysicalPosition<f64>>,
    gl_cursor: Option<GLCoord4D>,
    world_cursor: Option<WorldCoord>,
}

impl CursorHandler {
    pub fn new(physical_window_size: PhysicalSize<u32>) -> CursorHandler {
        CursorHandler {
            z_finder: GLZFinder::new(),
            physical_window_size,
            screen_cursor: None,
            gl_cursor: None,
            world_cursor: None,
        }
    }

    pub fn gl_cursor(&self) -> Option<GLCoord4D> {
        self.gl_cursor
    }

    pub fn world_cursor(&self) -> Option<WorldCoord> {
        self.world_cursor
    }

    fn get_gl_cursor(&self) -> Option<GLCoord4D> {
        self.screen_cursor
            .map(|position| position.to_gl_coord_4d(self.physical_window_size, &self.z_finder))
    }

    fn compute_world_cursor(&self, transform: &mut Transform) -> Option<WorldCoord> {
        self.gl_cursor
            .filter(|gl_coord| gl_coord.z < 1.0)
            .map(|gl_coord| gl_coord.to_world_coord(transform))
    }

    pub fn update_gl_and_world_cursor(&mut self, transform: &mut Transform) {
        self.gl_cursor = self.get_gl_cursor();
        self.world_cursor = self.compute_world_cursor(transform);
    }
}

impl EventConsumer for CursorHandler {
    fn consume_event(&mut self, event: Arc<Event>) {
        match *event {
            Event::GlutinEvent(glutin::event::Event::WindowEvent {
                event: glutin::event::WindowEvent::CursorMoved { position, .. },
                ..
            }) => {
                self.screen_cursor = Some(position);
            }
            Event::GlutinEvent(glutin::event::Event::WindowEvent {
                event: glutin::event::WindowEvent::Resized(physical_size),
                ..
            }) => {
                self.physical_window_size = physical_size;
            }
            _ => (),
        }
    }
}
