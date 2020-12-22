use engine::{Command, Event};
use events::EventHandler;
use std::sync::Arc;

#[derive(Default)]
pub struct DPIRelay {}

impl EventHandler for DPIRelay {
    fn handle_event(&mut self, event: Arc<Event>) -> Vec<Command> {
        match *event {
            Event::GlutinEvent(glutin::event::Event::WindowEvent {
                event: glutin::event::WindowEvent::ScaleFactorChanged { scale_factor, .. },
                ..
            }) => vec![Command::Event(Event::DPIChanged(scale_factor))],
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
}

impl EventHandler for ResizeRelay {
    fn handle_event(&mut self, event: Arc<Event>) -> Vec<Command> {
        match *event {
            Event::GlutinEvent(glutin::event::Event::WindowEvent {
                event: glutin::event::WindowEvent::Resized(physical_size),
                ..
            }) => vec![Command::Event(Event::Resize(
                glutin::dpi::PhysicalSize::new(
                    physical_size.width as f64,
                    physical_size.height as f64,
                ),
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
