use std::sync::Arc;

use commons::async_channel::{Receiver, RecvError};
use commons::async_trait::async_trait;
use commons::fn_sender::FnSender;
use commons::futures::executor::ThreadPool;
use commons::futures::future::RemoteHandle;
use commons::log::info;
use commons::FutureExt;
use isometric::{Button, ElementState, Event, ModifiersState, VirtualKeyCode};

use crate::actors::ObjectBuilder;
use crate::polysender::Polysender;

#[async_trait]
pub trait ActorTraits {
    async fn run(mut self) -> Self;
    fn resume(&mut self);
    fn shutdown(&mut self);
}

struct Actor<T>
where
    T: ActorTraits + Send + 'static,
{
    state: ActorState<T>,
}

enum ActorState<T>
where
    T: ActorTraits,
{
    Running(RemoteHandle<T>),
    Paused(Option<T>),
}

impl<T> Actor<T>
where
    T: ActorTraits + Send + Sync + 'static,
{
    fn start(&mut self, pool: &ThreadPool) {
        if let ActorState::Paused(actor) = &mut self.state {
            let mut actor = actor.take().unwrap();
            actor.resume();
            let (runnable, handle) = async move { actor.run().await }.remote_handle();
            pool.spawn_ok(runnable);
            self.state = ActorState::Running(handle)
        } else {
            panic!("Actor is not idle!");
        }
    }

    async fn pause(&mut self, sender: &FnSender<T>) {
        if let ActorState::Running(handle) = &mut self.state {
            sender.send(|actor| actor.shutdown()).await;
            self.state = ActorState::Paused(Some(handle.await));
        } else {
            panic!("Actor is not running!");
        }
    }
}

pub struct Reactor {
    x: Polysender,
    engine_rx: Receiver<Arc<Event>>,
    pool: ThreadPool,
    object_builder: Actor<ObjectBuilder<Polysender>>,
    bindings: ReactorBindings,
    paused: bool,
    run: bool,
}

struct ReactorBindings {
    pause: Button,
}

impl Reactor {
    pub fn new(
        x: Polysender,
        engine_rx: Receiver<Arc<Event>>,
        pool: ThreadPool,
        object_builder: ObjectBuilder<Polysender>,
    ) -> Reactor {
        Reactor {
            x,
            engine_rx,
            pool,
            object_builder: Actor {
                state: ActorState::Paused(Some(object_builder)),
            },
            bindings: ReactorBindings {
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

    async fn shutdown(&mut self) {
        info!("Shutting down reactor");
        self.pause().await;
        self.run = false;
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
        self.object_builder.pause(&self.x.object_builder_tx).await;
        self.paused = true;
        info!("Paused reactor");
    }
}
