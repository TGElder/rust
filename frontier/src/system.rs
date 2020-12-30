use std::sync::Arc;

use commons::async_channel::{Receiver, Sender};
use commons::async_trait::async_trait;
use commons::fn_sender::{FnMessageExt, FnReceiver, FnSender};
use commons::log::info;
use futures::executor::block_on;
use futures::executor::ThreadPool;
use futures::future::{FutureExt, RemoteHandle};
use isometric::{Button, ElementState, Event, EventConsumer, VirtualKeyCode};

const SAVE_PATH: &str = "save";

pub struct System<T> {
    pool: ThreadPool,
    listener: T,
    paused: bool,
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
    pub fn new(pool: ThreadPool, listener: T) -> System<T> {
        System {
            pool,
            listener,
            paused: false,
        }
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
    }
}

pub struct SystemRunner {
    pub handle: RemoteHandle<()>,
}

impl SystemRunner {
    pub fn run<T: Send + SystemListener + 'static>(
        mut system: System<T>,
        mut system_rx: FnReceiver<System<T>>,
        shutdown_rx: Receiver<()>,
    ) -> SystemRunner {
        let pool = system.pool.clone(); // TODO weird
        let (runnable, handle) = async move {
            system.start().await;
            loop {
                select! {
                mut message = shutdown_rx.recv().fuse() => {
                    system_rx.get_messages().apply(&mut system).await;
                    return;
                },
                mut message = system_rx.get_message().fuse() => message.apply(&mut system).await,}
            }
        }
        .remote_handle();

        pool.spawn_ok(runnable);
        SystemRunner { handle }
    }
}

pub struct SystemEventForwarder<T>
where
    T: Send,
{
    tx: FnSender<System<T>>,
    shutdown_tx: Sender<()>,
    bindings: Bindings,
}

impl<T> SystemEventForwarder<T>
where
    T: Send,
{
    pub fn new(tx: FnSender<System<T>>, shutdown_tx: Sender<()>) -> SystemEventForwarder<T> {
        SystemEventForwarder {
            tx,
            shutdown_tx,
            bindings: Bindings {
                pause: Button::Key(VirtualKeyCode::Space),
                save: Button::Key(VirtualKeyCode::P),
            },
        }
    }
}

impl<T> EventConsumer for SystemEventForwarder<T>
where
    T: Send + SystemListener,
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
            block_on(self.shutdown_tx.send(())).unwrap();
        }
    }
}
