use std::sync::Arc;

use commons::log::info;
use futures::executor::block_on;
use futures::FutureExt;
use isometric::{Button, ElementState, Event, EventConsumer, VirtualKeyCode};

use crate::traits::SendSystem;

const SAVE_PATH: &str = "save";

pub struct SystemController<T> {
    cx: T,
    bindings: Bindings,
    paused: bool,
}

struct Bindings {
    pause: Button,
    save: Button,
}

impl<T> SystemController<T>
where
    T: SendSystem,
{
    pub fn new(cx: T) -> SystemController<T> {
        SystemController {
            cx,
            bindings: Bindings {
                pause: Button::Key(VirtualKeyCode::Space),
                save: Button::Key(VirtualKeyCode::P),
            },
            paused: false,
        }
    }

    fn set_pause(&mut self, pause: bool) {
        if self.paused != pause {
            if pause {
                info!("Pausing system");
                block_on(self.cx.send_system_future(|system| system.pause().boxed()));
                info!("Paused system");
            } else {
                info!("Starting system");
                block_on(self.cx.send_system_future(|system| system.start().boxed()));
                info!("Started system");
            }
            self.paused = pause;
        }
    }

    fn toggle_pause(&mut self) {
        self.set_pause(!self.paused);
    }

    fn save(&mut self) {
        info!("Saving system");
        let was_paused = self.paused;
        self.set_pause(true);
        block_on(
            self.cx
                .send_system_future(|system| system.save(SAVE_PATH).boxed()),
        );
        self.set_pause(was_paused);
        info!("Saved system");
    }

    fn shutdown(&mut self) {
        info!("Shutting down system");
        self.set_pause(true);
        block_on(self.cx.send_system(|system| system.shutdown()));
        info!("Shut down system");
    }
}

impl<T> EventConsumer for SystemController<T>
where
    T: SendSystem + Send,
{
    fn consume_event(&mut self, event: Arc<Event>) {
        if let Event::Button {
            ref button,
            state: ElementState::Pressed,
            modifiers,
            ..
        } = *event
        {
            if button == &self.bindings.pause && !modifiers.alt() && modifiers.ctrl() {
                self.toggle_pause();
            } else if button == &self.bindings.save && !modifiers.alt() && modifiers.ctrl() {
                self.save();
            }
        }
        if let Event::Shutdown = *event {
            self.shutdown();
        }
    }
}
