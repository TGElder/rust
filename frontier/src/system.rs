use std::sync::Arc;

use commons::async_channel::{unbounded, Receiver, RecvError, Sender};
use commons::async_trait::async_trait;
use commons::fn_sender::FnSender;
use commons::log::info;
use futures::executor::block_on;
use futures::executor::ThreadPool;
use futures::future::FutureExt;
use isometric::{Button, ElementState, Event, EventConsumer, IsometricEngine, VirtualKeyCode};

const SAVE_PATH: &str = "save";

pub struct System<T> {
    pool: ThreadPool,
    listener: T,
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

        
    }
}

pub struct SystemEventForwarder<T> 
    where T: Send
{
    tx: FnSender<System<T>>,
    bindings: Bindings,
}

impl <T> SystemEventForwarder<T> 
    where T: Send
{
    pub fn new(tx: FnSender<System<T>>) -> SystemEventForwarder<T> {
        SystemEventForwarder { tx, bindings: Bindings {
            pause: Button::Key(VirtualKeyCode::Space),
            save: Button::Key(VirtualKeyCode::P),
        }, }
    }
}

impl <T> EventConsumer for SystemEventForwarder<T> 
    where T: Send + SystemListener
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
                self.tx.send_future(|system| system.toggle_pause().boxed());
            } else if button == &self.bindings.save && !modifiers.alt() && modifiers.ctrl() {
                self.tx.send_future(|system| system.save(SAVE_PATH).boxed());
            }
        }
        if let Event::Shutdown = *event {
            self.tx.send_future(|system| system.shutdown().boxed());
        }
    }
}
