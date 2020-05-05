use coords::{GLCoord4D, PhysicalPositionExt, WorldCoord};
use engine::Event;
use events::EventConsumer;
use glutin::dpi::{LogicalPosition, LogicalSize, PhysicalSize};
use graphics::GLZFinder;
use std::sync::Arc;
use transform::Transform;

pub struct CursorHandler {
    z_finder: GLZFinder,
    dpi_factor: f64,
    physical_window_size: PhysicalSize,
    screen_cursor: Option<LogicalPosition>,
    gl_cursor: Option<GLCoord4D>,
    world_cursor: Option<WorldCoord>,
}

impl CursorHandler {
    pub fn new(dpi_factor: f64, logical_window_size: LogicalSize) -> CursorHandler {
        CursorHandler {
            z_finder: GLZFinder {},
            dpi_factor,
            physical_window_size: logical_window_size.to_physical(dpi_factor),
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
        self.screen_cursor.map(|position| {
            position
                .to_physical(self.dpi_factor)
                .to_gl_coord_4d(self.physical_window_size, &self.z_finder)
        })
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
            Event::GlutinEvent(glutin::Event::WindowEvent {
                event: glutin::WindowEvent::CursorMoved { position, .. },
                ..
            }) => {
                self.screen_cursor = Some(position);
            }
            Event::DPIChanged(dpi) => {
                self.dpi_factor = dpi;
            }
            Event::Resize(physical_size) => {
                self.physical_window_size = physical_size;
            }
            _ => (),
        }
    }
}
