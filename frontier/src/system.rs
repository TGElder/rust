use std::sync::Arc;

use commons::async_channel::{Receiver, RecvError};
use commons::async_trait::async_trait;
use commons::fn_sender::{fn_channel, FnMessageExt, FnReceiver, FnSender};
use commons::futures::executor::ThreadPool;
use commons::futures::future::{FutureExt, RemoteHandle};
use commons::log::info;
use isometric::{Button, ElementState, Event, ModifiersState, VirtualKeyCode};

use crate::actors::ObjectBuilder;
use crate::polysender::Polysender;

pub struct Program<T>
where
    T: Send,
{
    actor: T,
    actor_rx: FnReceiver<T>,
    tx: FnSender<Program<T>>,
    rx: FnReceiver<Program<T>>,
    engine_rx: Receiver<Arc<Event>>,
    run: bool,
}

impl<T> Program<T>
where
    T: HandleEngineEvents<T> + Send,
{
    pub fn new(actor: T, rx: FnReceiver<T>, engine_rx: Receiver<Arc<Event>>) -> Program<T> {
        let (ptx, prx) = fn_channel();
        Program {
            actor,
            actor_rx: rx,
            tx: ptx,
            rx: prx,
            engine_rx,
            run: true,
        }
    }

    pub fn tx(&self) -> &FnSender<Program<T>> {
        &self.tx
    }

    pub async fn run(mut self) -> Self {
        while self.run {
            self.step().await;
        }
        self.run = true;
        self
    }

    async fn step(&mut self)
    where
        T: HandleEngineEvents<T> + Send,
    {
        select! {
            mut message = self.rx.get_message().fuse() => message.apply(self).await,
            mut message = self.actor_rx.get_message().fuse() => message.apply(&mut self.actor).await,
            event = self.engine_rx.recv().fuse() => handle_engine_event(&mut self.actor, event).await
        }
    }

    pub fn shutdown(&mut self) {
        self.run = false;
    }
}

#[async_trait]
pub trait HandleEngineEvents<T> {
    async fn handle_engine_event(&mut self, event: Arc<Event>);
}

async fn handle_engine_event<T>(t: &mut T, event: Result<Arc<Event>, RecvError>)
where
    T: HandleEngineEvents<T> + Send,
{
    let event = event.unwrap(); // TODO better message
    t.handle_engine_event(event).await;
}

struct Process<T>
where
    T: HandleEngineEvents<T> + Send + 'static,
{
    state: ProcessState<Program<T>>,
}

enum ProcessState<T>
where
    T: Send,
{
    Running {
        handle: RemoteHandle<T>,
        tx: FnSender<T>,
    },
    Paused(Option<T>),
}

impl<T> Process<T>
where
    T: HandleEngineEvents<T> + Send + Sync + 'static,
{
    fn start(&mut self, pool: &ThreadPool) {
        if let ProcessState::Paused(actor) = &mut self.state {
            let actor = actor.take().unwrap();
            let tx = actor.tx().clone();
            let (runnable, handle) = async move { actor.run().await }.remote_handle();
            pool.spawn_ok(runnable);
            self.state = ProcessState::Running { handle, tx };
        } else {
            panic!("Actor is not idle!");
        }
    }

    async fn pause(&mut self) {
        if let ProcessState::Running { handle, tx } = &mut self.state {
            tx.send(|actor| actor.shutdown()).await;
            self.state = ProcessState::Paused(Some(handle.await));
        } else {
            panic!("Actor is not running!");
        }
    }
}

pub struct System {
    x: Polysender,
    engine_rx: Receiver<Arc<Event>>,
    pool: ThreadPool,
    object_builder: Process<ObjectBuilder<Polysender>>,
    bindings: SystemBindings,
    paused: bool,
    run: bool,
}

struct SystemBindings {
    pause: Button,
}

impl System {
    pub fn new(
        x: Polysender,
        engine_rx: Receiver<Arc<Event>>,
        pool: ThreadPool,
        object_builder: Program<ObjectBuilder<Polysender>>,
    ) -> System {
        System {
            x,
            engine_rx,
            pool,
            object_builder: Process {
                state: ProcessState::Paused(Some(object_builder)),
            },
            bindings: SystemBindings {
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
        info!("Shut down reactor");
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
        info!("Starting reactor");
        self.object_builder.start(&self.pool);
        self.paused = false;
        info!("Started reactor");
    }

    async fn toggle_pause(&mut self) {
        if self.paused {
            self.start();
        } else {
            self.pause().await;
        }
    }

    async fn pause(&mut self) {
        info!("Pausing reactor");
        self.object_builder.pause().await;
        self.paused = true;
        info!("Paused reactor");
    }

    async fn shutdown(&mut self) {
        info!("Shutting down reactor");
        self.pause().await;
        self.run = false;
    }
}
