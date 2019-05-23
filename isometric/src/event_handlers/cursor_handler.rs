use coords::PhysicalPositionExt;
use engine::{Command, Event};
use events::EventHandler;
use graphics::GLZFinder;
use std::sync::Arc;

pub struct CursorHandler {
    z_finder: GLZFinder,
    dpi_factor: f64,
    physical_window_size: glutin::dpi::PhysicalSize,
    cursor_position: Option<glutin::dpi::LogicalPosition>,
}

impl CursorHandler {
    pub fn new(dpi_factor: f64, logical_window_size: glutin::dpi::LogicalSize) -> CursorHandler {
        CursorHandler {
            z_finder: GLZFinder {},
            dpi_factor,
            physical_window_size: logical_window_size.to_physical(dpi_factor),
            cursor_position: None,
        }
    }

    fn handle_move(&mut self, position: glutin::dpi::LogicalPosition) -> Vec<Command> {
        self.cursor_position = Some(position);
        vec![]
    }

    fn handle_draw(&self) -> Vec<Command> {
        if let Some(position) = self.cursor_position {
            let gl_coord = position
                .to_physical(self.dpi_factor)
                .to_gl_coord_4d(self.physical_window_size, &self.z_finder);
            vec![
                Command::Event(Event::CursorMoved(gl_coord)),
                Command::ComputeWorldPosition(gl_coord),
            ]
        } else {
            vec![]
        }
    }
}

impl EventHandler for CursorHandler {
    fn handle_event(&mut self, event: Arc<Event>) -> Vec<Command> {
        match *event {
            Event::GlutinEvent(glutin::Event::WindowEvent {
                event: glutin::WindowEvent::CursorMoved { position, .. },
                ..
            }) => self.handle_move(position),
            Event::DPIChanged(dpi) => {
                self.dpi_factor = dpi;
                vec![]
            }
            Event::Resize(physical_size) => {
                self.physical_window_size = physical_size;
                vec![]
            }
            Event::WorldDrawn => self.handle_draw(),
            _ => vec![],
        }
    }
}
