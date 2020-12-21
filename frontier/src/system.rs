use std::sync::Arc;

use commons::async_channel::{unbounded, Receiver, RecvError, Sender};
use commons::async_trait::async_trait;
use commons::log::info;
use futures::executor::block_on;
use futures::executor::ThreadPool;
use futures::future::FutureExt;
use isometric::{
    Button, ElementState, Event, EventConsumer, IsometricEngine, ModifiersState, VirtualKeyCode,
};

const SAVE_PATH: &str = "save";

pub struct System<T> {
    engine_rx: Receiver<Arc<Event>>,
    pool: ThreadPool,
    listener: T,
    bindings: Bindings,
    paused: bool,
    run: bool,
}

#[async_trait]
pub trait SystemListener {
    async fn start(&mut self, pool: &ThreadPool);
    async fn pause(&mut self, pool: &ThreadPool);
    async fn save(&mut self, path: &str);
}

struct Bindings {
    pause: Button,
    save: Button,
}

impl<T> System<T>
where
    T: SystemListener,
{
    pub fn new(engine: &mut IsometricEngine, pool: ThreadPool, listener: T) -> System<T> {
        let (engine_tx, engine_rx) = unbounded();
        engine.add_event_consumer(SystemEventForwarder::new(engine_tx));

        System {
            engine_rx,
            pool,
            listener,
            bindings: Bindings {
                pause: Button::Key(VirtualKeyCode::Space),
                save: Button::Key(VirtualKeyCode::P),
            },
            paused: false,
            run: true,
        }
    }

    pub async fn run(&mut self) {
        self.start().await;
        while self.run {
            select! {
                event = self.engine_rx.recv().fuse() => self.handle_engine_event(event).await
            }
        }
        info!("Shut down system");
    }

    async fn start(&mut self) {
        info!("Starting system");
        self.listener.start(&self.pool).await;
        self.paused = false;
        info!("Started system");
    }

    async fn pause(&mut self) {
        info!("Pausing system");
        self.listener.pause(&self.pool).await;
        self.paused = true;
        info!("Paused system");
    }

    async fn toggle_pause(&mut self) {
        if self.paused {
            self.start().await;
        } else {
            self.pause().await;
        }
    }
    async fn save(&mut self, path: &str) {
        info!("Saving");
        let already_paused = self.paused;
        if !already_paused {
            self.pause().await;
        }

        self.listener.save(path).await;

        if !already_paused {
            self.start().await;
        }
        info!("Saved");
    }

    async fn shutdown(&mut self) {
        info!("Shutting down system");
        if !self.paused {
            self.pause().await;
        }
        self.run = false;
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
            } else if button == &self.bindings.save {
                self.save(SAVE_PATH).await;
            }
        }
        if let Event::Shutdown = *event {
            self.shutdown().await;
        }
    }
}

pub struct SystemEventForwarder {
    tx: Sender<Arc<Event>>,
}

impl SystemEventForwarder {
    pub fn new(tx: Sender<Arc<Event>>) -> SystemEventForwarder {
        SystemEventForwarder { tx }
    }
}

impl EventConsumer for SystemEventForwarder {
    fn consume_event(&mut self, event: Arc<Event>) {
        block_on(async { self.tx.send(event.clone()).await }).unwrap();
    }
}
