use std::sync::Arc;

use commons::async_channel::Sender;
use commons::fn_sender::FnSender;
use commons::log::info;
use futures::executor::block_on;
use futures::future::FutureExt;
use isometric::{Button, ElementState, Event, EventConsumer, VirtualKeyCode};

use crate::system::System;

const SAVE_PATH: &str = "save";

pub struct SystemController {
    cx: FnSender<System>,
    shutdown_tx: Sender<()>,
    bindings: Bindings,
    paused: bool,
}

struct Bindings {
    pause: Button,
    save: Button,
}

impl SystemController {
    pub fn new(cx: FnSender<System>, shutdown_tx: Sender<()>) -> SystemController {
        SystemController {
            cx,
            shutdown_tx,
            bindings: Bindings {
                pause: Button::Key(VirtualKeyCode::Space),
                save: Button::Key(VirtualKeyCode::P),
            },
            paused: false,
        }
    }
}

impl SystemController {
    fn set_pause(&mut self, pause: bool) {
        if self.paused != pause {
            if pause {
                info!("Pausing system");
                block_on(self.cx.send_future(|system| system.pause().boxed()));
                info!("Paused system");
            } else {
                info!("Starting system");
                block_on(self.cx.send_future(|system| system.start().boxed()));
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
        block_on(self.cx.send_future(|system| system.save(SAVE_PATH).boxed()));
        self.set_pause(was_paused);
        info!("Saved system");
    }

    fn shutdown(&mut self) {
        info!("Shutting down system");
        self.set_pause(true);
        block_on(self.shutdown_tx.send(())).unwrap();
    }
}

impl EventConsumer for SystemController {
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
