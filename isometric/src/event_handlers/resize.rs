use engine::{Command, Event};
use events::EventHandler;
use std::sync::Arc;

#[derive(Default)]
pub struct DPIRelay {}

impl EventHandler for DPIRelay {
    fn handle_event(&mut self, event: Arc<Event>) -> Vec<Command> {
        match *event {
            Event::GlutinEvent(glutin::Event::WindowEvent {
                event: glutin::WindowEvent::HiDpiFactorChanged(dpi_factor),
                ..
            }) => vec![Command::Event(Event::DPIChanged(dpi_factor))],
            _ => vec![],
        }
    }
}

pub struct ResizeRelay {
    dpi_factor: f64,
}

impl ResizeRelay {
    pub fn new(dpi_factor: f64) -> ResizeRelay {
        ResizeRelay { dpi_factor }
    }

    fn get_physical_size(
        &self,
        logical_size: glutin::dpi::LogicalSize,
    ) -> glutin::dpi::PhysicalSize {
        logical_size.to_physical(self.dpi_factor)
    }
}

impl EventHandler for ResizeRelay {
    fn handle_event(&mut self, event: Arc<Event>) -> Vec<Command> {
        match *event {
            Event::GlutinEvent(glutin::Event::WindowEvent {
                event: glutin::WindowEvent::Resized(logical_size),
                ..
            }) => vec![Command::Event(Event::Resize(
                self.get_physical_size(logical_size),
            ))],
            Event::DPIChanged(dpi) => {
                self.dpi_factor = dpi;
                vec![]
            }
            _ => vec![],
        }
    }
}

#[derive(Default)]
pub struct Resizer {}

impl EventHandler for Resizer {
    fn handle_event(&mut self, event: Arc<Event>) -> Vec<Command> {
        match *event {
            Event::Resize(physical_size) => vec![Command::Resize(physical_size)],
            _ => vec![],
        }
    }
}
