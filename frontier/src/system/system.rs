use std::sync::Arc;

use commons::async_channel::{Receiver, RecvError};
use commons::futures::executor::ThreadPool;
use commons::futures::future::FutureExt;
use commons::log::info;
use isometric::{Button, ElementState, Event, ModifiersState, VirtualKeyCode};

use crate::actors::{ObjectBuilder, Voyager};
use crate::polysender::Polysender;
use crate::system::{Process, Program};

pub struct System {
    engine_rx: Receiver<Arc<Event>>,
    pool: ThreadPool,
    processes: Processes,
    bindings: Bindings,
    paused: bool,
    run: bool,
}

struct Processes {
    object_builder: Process<ObjectBuilder<Polysender>>,
    voyager: Process<Voyager<Polysender>>,
}

pub struct Programs {
    pub object_builder: Program<ObjectBuilder<Polysender>>,
    pub voyager: Program<Voyager<Polysender>>,
}

impl Into<Processes> for Programs {
    fn into(self) -> Processes {
        Processes {
            object_builder: Process::new(self.object_builder),
            voyager: Process::new(self.voyager),
        }
    }
}

struct Bindings {
    pause: Button,
}

impl System {
    pub fn new(engine_rx: Receiver<Arc<Event>>, pool: ThreadPool, programs: Programs) -> System {
        System {
            engine_rx,
            pool,
            processes: programs.into(),
            bindings: Bindings {
                pause: Button::Key(VirtualKeyCode::Space),
            },
            paused: false,
            run: true,
        }
    }

    pub async fn run(&mut self) {
        self.start();
        while self.run {
            select! {
                event = self.engine_rx.recv().fuse() => self.handle_engine_event(event).await
            }
        }
        info!("Shut down system");
    }

    async fn handle_engine_event(&mut self, event: Result<Arc<Event>, RecvError>) {
        let event: Arc<Event> = event.unwrap();

        if let Event::Button {
            ref button,
            state: ElementState::Pressed,
            modifiers:
                ModifiersState {
                    alt: false,
                    ctrl: true,
                    ..
                },
            ..
        } = *event
        {
            if button == &self.bindings.pause {
                self.toggle_pause().await;
            }
        }
        if let Event::Shutdown = *event {
            self.shutdown().await;
        }
    }

    fn start(&mut self) {
        info!("Starting system");
        self.processes.object_builder.start(&self.pool);
        self.processes.voyager.start(&self.pool);
        self.paused = false;
        info!("Started system");
    }

    async fn toggle_pause(&mut self) {
        if self.paused {
            self.start();
        } else {
            self.pause().await;
        }
    }

    async fn pause(&mut self) {
        info!("Pausing system");
        join!(
            self.processes.object_builder.pause(),
            self.processes.voyager.pause(),
        );
        self.paused = true;
        info!("Paused system");
    }

    async fn shutdown(&mut self) {
        info!("Shutting down system");
        if !self.paused {
            self.pause().await;
        }
        self.run = false;
    }
}
