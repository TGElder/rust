use std::{sync::Arc, time::Duration};

use commons::{async_std::task::sleep, async_trait::async_trait, process::Step};
use isometric::{Button, ElementState, Event, VirtualKeyCode};

use crate::system::{Capture, HandleEngineEvent};

pub struct RiverExplorer<T> {
    cx: T,
    active: bool,
    parameters: RiverExplorerParameters,
}

pub struct RiverExplorerParameters {
    pub refresh_interval: Duration,
    pub binding: Button,
}

impl Default for RiverExplorerParameters {
    fn default() -> RiverExplorerParameters {
        RiverExplorerParameters {
            refresh_interval: Duration::from_millis(100),
            binding: Button::Key(VirtualKeyCode::X),
        }
    }
}

impl<T> RiverExplorer<T> {
    pub fn new(cx: T, parameters: RiverExplorerParameters) -> RiverExplorer<T> {
        RiverExplorer {
            cx,
            active: false,
            parameters,
        }
    }

    async fn explore(&self) {}
}

#[async_trait]
impl<T> Step for RiverExplorer<T>
where
    T: Send + Sync,
{
    async fn step(&mut self) {
        if self.active {
            self.explore().await;
        }

        sleep(self.parameters.refresh_interval).await;
    }
}

#[async_trait]
impl<T> HandleEngineEvent for RiverExplorer<T>
where
    T: Send + Sync + 'static,
{
    async fn handle_engine_event(&mut self, event: Arc<Event>) -> Capture {
        if let Event::Button {
            ref button,
            state: ElementState::Pressed,
            modifiers,
            ..
        } = *event
        {
            if *button == self.parameters.binding && !modifiers.alt() && modifiers.ctrl() {
                self.active = !self.active;
            }
        }
        Capture::No
    }
}
