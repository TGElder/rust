mod active_process;
mod passive_process;

use commons::async_trait::async_trait;
use commons::fn_sender::{fn_channel, FnMessageExt, FnReceiver, FnSender};
use futures::executor::ThreadPool;
use futures::future::{FutureExt, RemoteHandle};

use std::any::type_name;

pub use active_process::*;
use commons::log::{debug, error};
pub use passive_process::*;

pub struct Program<T>
where
    T: Send,
{
    actor: T,
    actor_rx: FnReceiver<T>,
    tx: FnSender<Self>,
    rx: FnReceiver<Self>,
    run: bool,
}

impl<T> Program<T>
where
    T: Send,
{
    fn new(actor: T, actor_rx: FnReceiver<T>) -> Program<T> {
        let (tx, rx) = fn_channel();
        Program {
            actor,
            actor_rx,
            tx,
            rx,
            run: true,
        }
    }

    async fn manually_process_actor_messages(&mut self)
    where
        T: Send,
    {
        let mut messages = self.actor_rx.get_messages();
        if !messages.is_empty() {
            debug!(
                "{} messages from {:?} manually processed",
                messages.len(),
                type_name::<T>()
            );
            messages.apply(&mut self.actor).await;
        }
    }
}

pub struct Drain<T> {
    actor_rx: FnReceiver<T>,
    tx: FnSender<Self>,
    rx: FnReceiver<Self>,
    run: bool,
    count: usize,
}

impl<T> Drain<T> {
    fn new(actor_rx: FnReceiver<T>) -> Drain<T> {
        let (tx, rx) = fn_channel();
        Drain {
            actor_rx,
            tx,
            rx,
            run: true,
            count: 0,
        }
    }

    async fn run(mut self) -> Self {
        while self.run {
            select! {
                mut message = self.rx.get_message().fuse() => message.apply(&mut self).await,
                mut message = self.actor_rx.get_message().fuse() => self.count += 1,
            }
        }
        self
    }
}

pub enum ReceiverState<T>
where
    T: Send,
{
    Accumulating {
        actor_rx: FnReceiver<T>,
    },
    Draining {
        tx: FnSender<Drain<T>>,
        handle: RemoteHandle<Drain<T>>,
    },
}

impl<T> ReceiverState<T>
where
    T: Send + 'static,
{
    fn accumulating(actor_rx: FnReceiver<T>) -> ReceiverState<T> {
        ReceiverState::Accumulating { actor_rx }
    }

    fn draining(actor_rx: FnReceiver<T>, pool: &ThreadPool) -> ReceiverState<T> {
        let drain = Drain::new(actor_rx);

        let tx = drain.tx.clone();

        let (runnable, handle) = async move { drain.run().await }.remote_handle();
        pool.spawn_ok(runnable);

        ReceiverState::Draining { tx, handle }
    }

    async fn actor_rx(self) -> FnReceiver<T> {
        match self {
            ReceiverState::Accumulating { actor_rx } => actor_rx,
            ReceiverState::Draining { tx, handle } => {
                async {
                    tx.send(|drain| drain.run = false).await;
                    let drain = handle.await;
                    if drain.count > 0 {
                        error!(
                            "{} messages for {:?} were drained!",
                            drain.count,
                            type_name::<T>()
                        );
                    }
                    drain.actor_rx
                }
                .await
            }
        }
    }
}

pub enum ProcessState<T>
where
    T: Send,
{
    Running {
        tx: FnSender<Program<T>>,
        handle: RemoteHandle<(T, FnReceiver<T>)>,
    },
    Paused {
        actor: T,
        rx_state: ReceiverState<T>,
    },
}

#[async_trait]
pub trait Process: Send {
    type T: Send + 'static;

    fn state(&self) -> &Option<ProcessState<Self::T>>;
    fn mut_state(&mut self) -> &mut Option<ProcessState<Self::T>>;
    async fn step(t: &mut Program<Self::T>);

    async fn run(mut t: Program<Self::T>) -> (Self::T, FnReceiver<Self::T>) {
        while t.run {
            Self::step(&mut t).await;
        }

        t.manually_process_actor_messages().await;

        t.run = true;
        (t.actor, t.actor_rx)
    }

    async fn start(&mut self, pool: &ThreadPool) {
        let state = self.mut_state().take().unwrap();
        if let ProcessState::Paused {
            actor,
            rx_state: receiver,
        } = state
        {
            let actor_rx = receiver.actor_rx().await;

            let program = Program::new(actor, actor_rx);
            let tx = program.tx.clone();

            let (runnable, handle) = async move { Self::run(program).await }.remote_handle();
            pool.spawn_ok(runnable);
            *self.mut_state() = Some(ProcessState::Running { handle, tx });
            debug!("Started {:?}", type_name::<Self::T>());
        } else {
            panic!("Cannot start program: program is not paused!");
        }
    }

    async fn pause(&mut self, pool: &ThreadPool) {
        if let ProcessState::Running { handle, tx } = self.mut_state().take().unwrap() {
            tx.send(|program| program.run = false).await;
            let (actor, actor_rx) = handle.await;

            *self.mut_state() = Some(ProcessState::Paused {
                actor,
                rx_state: ReceiverState::draining(actor_rx, pool),
            });
            debug!("Paused {:?}", type_name::<Self::T>());
        } else {
            panic!("Cannot pause program: program is not running!");
        }
    }
}

pub trait Persistable {
    fn save(&self, path: &str);
    fn load(&mut self, path: &str);
}

impl<T> Persistable for T
where
    T: Process,
    <T as Process>::T: Persistable,
{
    fn save(&self, path: &str) {
        if let Some(ProcessState::Paused { actor, .. }) = self.state() {
            actor.save(path);
            debug!("Saved {:?}", type_name::<<T as Process>::T>());
        } else {
            panic!("Cannot save program state: program is not paused!");
        }
    }

    fn load(&mut self, path: &str) {
        if let Some(ProcessState::Paused { actor, .. }) = self.mut_state() {
            actor.load(path);
            debug!("Loaded {:?}", type_name::<<T as Process>::T>());
        } else {
            panic!("Cannot load program state: program is not paused!");
        }
    }
}
