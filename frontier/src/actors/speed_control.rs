use std::sync::Arc;

use crate::system::{Capture, HandleEngineEvent};
use crate::traits::WithClock;

use commons::async_trait::async_trait;
use isometric::{Button, ElementState, Event, VirtualKeyCode};

pub struct SpeedControlBindings {
    slow_down: Button,
    speed_up: Button,
}

impl Default for SpeedControlBindings {
    fn default() -> SpeedControlBindings {
        SpeedControlBindings {
            slow_down: Button::Key(VirtualKeyCode::Comma),
            speed_up: Button::Key(VirtualKeyCode::Period),
        }
    }
}

pub struct SpeedControl<T> {
    cx: T,
    bindings: SpeedControlBindings,
}

impl<T> SpeedControl<T>
where
    T: WithClock,
{
    pub fn new(cx: T) -> SpeedControl<T> {
        SpeedControl {
            cx,
            bindings: SpeedControlBindings::default(),
        }
    }

    async fn slow_down(&mut self) {
        self.cx.mut_clock(|clock| clock.adjust_speed(0.5)).await;
    }

    async fn speed_up(&mut self) {
        self.cx.mut_clock(|clock| clock.adjust_speed(2.0)).await;
    }
}

#[async_trait]
impl<T> HandleEngineEvent for SpeedControl<T>
where
    T: WithClock + Send + Sync,
{
    async fn handle_engine_event(&mut self, event: Arc<Event>) -> Capture {
        if let Event::Button {
            ref button,
            state: ElementState::Pressed,
            ..
        } = *event
        {
            if button == &self.bindings.slow_down {
                self.slow_down().await;
            }
            if button == &self.bindings.speed_up {
                self.speed_up().await;
            }
        }
        Capture::No
    }
}
