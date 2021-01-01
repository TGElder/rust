use std::sync::Arc;

use commons::async_channel::Sender;
use commons::fn_sender::FnSender;
use futures::executor::block_on;
use futures::future::FutureExt;
use isometric::{Button, ElementState, Event, EventConsumer, VirtualKeyCode};

use crate::system::System;

const SAVE_PATH: &str = "save";

pub struct SystemController {
    tx: FnSender<System>,
    shutdown_tx: Sender<()>,
    bindings: Bindings,
    paused: bool,
}

struct Bindings {
    pause: Button,
    save: Button,
}

impl SystemController {
    pub fn new(tx: FnSender<System>, shutdown_tx: Sender<()>) -> SystemController {
        SystemController {
            tx,
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
                self.tx.send_future(|system| system.pause().boxed());
            } else {
                self.tx.send_future(|system| system.start().boxed());
            }
            self.paused = pause;
        }
    }

    fn toggle_pause(&mut self) {
        self.set_pause(!self.paused);
    }

    fn save(&mut self) {
        let was_paused = self.paused;
        self.set_pause(true);
        self.tx.send_future(|system| system.save(SAVE_PATH).boxed());
        self.set_pause(was_paused);
    }

    fn shutdown(&mut self) {
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
